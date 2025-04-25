use std::{collections::BTreeMap, fmt::Display, sync::Arc};

use base16ct::lower::encode_string;
use chrono::{DateTime, Duration, Utc};
use futures::{stream, StreamExt};
use lazy_regex::{lazy_regex, Lazy};
use md5::{Digest, Md5};
use regex::Regex;
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde_json::Value;
use teloxide::{
    types::InputFile,
    utils::markdown::{bold, escape, link}
};
use tokio::sync::{Mutex, OnceCell};
use url::Url;

use super::{APIClient, LiveState, Metadata, API};
use crate::{
    platform::{Platform, User},
    subscription::Subscription
};

struct Wbi {
    client: APIClient,
    update_time: DateTime<Utc>,
    key: Option<String>
}

impl Wbi {
    const WBI_REGEX: Lazy<Regex> = lazy_regex!(r"\/wbi\/(.+)\.png");
    const KEY_MAP: [usize; 64] = [
        46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35,
        27, 43, 5, 49, 33, 9, 42, 19, 29, 28, 14, 39, 12, 38, 41, 13,
        37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
        22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52
    ];
    const KEY_LENGTH: usize = 32;

    fn new() -> Self {
        let headers = HeaderMap::from_iter([
            (header::USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36 Edg/132.0.0.0"))
        ]);
        Self {
            client: APIClient::new("https://api.bilibili.com/x/web-interface/nav".parse().unwrap(), headers, None),
            update_time: Utc::now(),
            key: None
        }
    }

    async fn update_key(&mut self) -> Option<()> {
        if self.key.is_none() || self.update_time + Duration::hours(2) > Utc::now() {
            let data = self.client.get::<()>(&[], None).await?;
            let img_url = data["data"]["wbi_img"]["img_url"].as_str()?;
            let sub_url = data["data"]["wbi_img"]["sub_url"].as_str()?;
            let img = &Wbi::WBI_REGEX.captures(img_url)?[1];
            let sub = &Wbi::WBI_REGEX.captures(sub_url)?[1];
            let full = img.to_owned() + sub;
            let full = full.as_bytes();
            let mut key = [0u8; Wbi::KEY_LENGTH];
            for i in 0..Wbi::KEY_LENGTH {
                key[i] = full[Wbi::KEY_MAP[i]];
            }
            self.key = Some(str::from_utf8(&key).ok()?.to_owned());
            self.update_time = Utc::now();
        }
        Some(())
    }

    async fn sign(&mut self, data: &mut BTreeMap<&str, String>) -> Option<()> {
        self.update_key().await?;
        let key = self.key.clone()?;
        let wts = Utc::now().timestamp();
        data.insert("wts", wts.to_string());
        let param = serde_urlencoded::to_string(&*data).ok()?;
        let hash = encode_string(&Md5::digest(param + &key));
        data.insert("w_rid", hash);
        Some(())
    }
}

static WBI: OnceCell<Arc<Mutex<Wbi>>> = OnceCell::const_new();

async fn get_wbi() -> Arc<Mutex<Wbi>> {
    WBI.get_or_init(|| async { Arc::new(Mutex::new(Wbi::new())) }).await.to_owned()
}

pub struct BilibiliAPI {
    client: APIClient
}

#[allow(dead_code)]
pub struct BilibiliLive {
    pub id: u64,
    pub url: Url,
    pub title: String,
    pub creator_name: String,
    pub creator_id: u64,
    pub cover_image_url: Url,
    pub start_time: DateTime<Utc>,
    pub state: LiveState
}

impl Metadata for BilibiliLive {
    type Id = u64;

    fn get_id(&self) -> &Self::Id {
        &self.id
    }

    fn get_state(&self) -> &LiveState {
        &self.state
    }

    fn get_attachment(&self) -> InputFile {
        InputFile::url(self.cover_image_url.clone())
    }

    fn to_sub(&self) -> Subscription {
        Subscription {
            platform: Platform::BilibiliLive,
            user: User { id: self.id.to_string(), username: self.creator_name.clone() }
        }
    }
}

impl BilibiliAPI {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.append(
            header::USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36 Edg/132.0.0.0")
        );
        Self { client: APIClient::new("https://api.live.bilibili.com".parse().unwrap(), headers, None) }
    }

    async fn get_info_by_room(&self, room_id: u64) -> Option<Value> {
        let path = "/xlive/web-room/v1/index/getInfoByRoom";
        let mut params = BTreeMap::from([
            ("room_id", room_id.to_string())
        ]);
        let wbi = get_wbi().await;
        wbi.lock().await.sign(&mut params).await?;
        let result = self.client.get(&[path], Some(params)).await?;
        if result["code"].as_i64()? != 0 {
            log::error!("Bilibili API error: {}", result["code"]);
            return None;
        }
        Some(result)
    }

    pub async fn username(&self, room_id: &String) -> Option<String> {
        let result = self.get_info_by_room(room_id.parse().unwrap()).await?;
        Some(result["data"]["anchor_info"]["base_info"]["uname"].as_str()?.to_owned())
    }
}

impl API<BilibiliLive> for BilibiliAPI {
    async fn live_status(&self, live_id: &String, _language: Option<String>) -> Option<BilibiliLive> {
        let result = self.get_info_by_room(live_id.parse().unwrap()).await?;
        let info = result["data"]["room_info"].as_object()?;
        let id = info["room_id"].as_u64()?;
        Some(BilibiliLive {
            id,
            url: format!("https://live.bilibili.com/{id}").parse().unwrap(),
            title: info["title"].as_str()?.to_owned(),
            creator_name: result["data"]["anchor_info"]["base_info"]["uname"].as_str()?.to_owned(),
            creator_id: info["uid"].as_u64()?,
            cover_image_url: info["cover"].as_str()?.parse().ok()?,
            start_time: DateTime::from_timestamp(info["live_start_time"].as_i64()?, 0)?,
            state: match info["live_status"].as_u64()? {
                0 => LiveState::Ended,
                1 => LiveState::Running,
                status => LiveState::Unknown(status.to_string())
            }
        })
    }

    async fn user_live_status(&self, subs: Vec<Subscription>) -> Vec<BilibiliLive> {
        stream::iter(subs).filter_map(
            async |sub| self.live_status(&sub.user.id, None).await.filter(|live| matches!(live.state, LiveState::Running))
        ).collect().await
    }
}

impl Display for BilibiliLive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.state {
            LiveState::Running => write!(
                f,
                "{} \\({}\\)'s Bilibili Live started\n{}",
                bold(escape(self.creator_name.as_str()).as_str()),
                link(format!("https://space.bilibili.com/{}", self.creator_id).as_str(), self.creator_id.to_string().as_str()),
                link(self.url.as_str(), escape(self.title.as_str()).as_str())
            ),
            LiveState::Ended => write!(
                f,
                "{} \\({}\\)'s Bilibili Live ended",
                bold(escape(self.creator_name.as_str()).as_str()),
                link(format!("https://space.bilibili.com/{}", self.creator_id).as_str(), self.creator_id.to_string().as_str()),
            ),
            LiveState::TimedOut => unreachable!(),
            LiveState::Unknown(state) => f.write_str(escape(format!("Unknown live state: {state}").as_str()).as_str())
        }
    }
}
