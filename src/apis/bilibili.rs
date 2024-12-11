use std::fmt::Display;

use chrono::{DateTime, Utc};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use teloxide::{types::InputFile, utils::markdown::{bold, escape, link}};
use url::Url;

use crate::{platform::{Platform, User}, subscription::Subscription};

use super::{APIClient, LiveState, Metadata, API};

pub struct BilibiliAPI {
    client: APIClient
}

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

#[derive(Serialize, Deserialize)]
struct GetInfoByRoomParams {
    room_id: u64
}

impl BilibiliAPI {
    pub fn new() -> Self {
        let mut headers = HeaderMap::new();
        headers.append(
            header::USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
        );
        Self { client: APIClient::new("https://api.live.bilibili.com".parse().unwrap(), headers, None) }
    }

    async fn get_info_by_room(&self, room_id: u64) -> Option<Value> {
        let path = "/xlive/web-room/v1/index/getInfoByRoom";
        let params = GetInfoByRoomParams { room_id };
        self.client.get(&[path], Some(params)).await
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
            cover_image_url: info["cover"].as_str()?.parse().unwrap(),
            start_time: DateTime::from_timestamp(info["live_start_time"].as_i64()?, 0)?,
            state: match info["live_status"].as_u64()? {
                0 => LiveState::Ended,
                1 => LiveState::Running,
                status => LiveState::Unknown(status.to_string())
            }
        })
    }

    async fn user_live_status(&self, subs: Vec<Subscription>) -> Vec<BilibiliLive> {
        let mut lives = vec![];
        for room_id in subs.iter().map(|sub| &sub.user.id) {
            if let Some(live) = self.live_status(room_id, None).await {
                match live.state {
                    LiveState::Running => lives.push(live),
                    _ => ()
                }
            }
        }
        lives
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
