use core::fmt;

use chrono::{DateTime, Utc};
use reqwest::{header::{self, HeaderMap, HeaderValue}, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use teloxide::utils::markdown::{bold, escape, link};
use url::Url;

use crate::subscription::Subscription;

use super::LiveState;

pub struct API {
    client: Client,
    live_api: Url
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

#[derive(Serialize, Deserialize)]
struct GetInfoByRoomParams {
    room_id: u64
}

impl API {
    pub fn new() -> API {
        let live_api: Url = "https://api.live.bilibili.com".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.append(
            header::USER_AGENT,
            HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36")
        );
        let client = Client::builder().default_headers(headers).build().unwrap();
        API { client, live_api }
    }

    async fn get<T: Serialize>(&self, path: &str, params: T) -> Option<Value> {
        let url = self.live_api.join(path).unwrap();
        let Ok(res) = self.client.get(url.clone()).query(&params).send().await else {
            log::error!("API error");
            return None;
        };
        if res.status().is_success() {
            let Ok(data) = res.json::<Value>().await else {
                log::error!("JSON decode error");
                return None;
            };
            return Some(data);
        }
        log::error!("{}: {}: {:?}", url, res.status(), res);
        None
    }

    async fn get_info_by_room(&self, room_id: u64) -> Option<Value> {
        let path = "/xlive/web-room/v1/index/getInfoByRoom";
        let params = GetInfoByRoomParams { room_id };
        self.get(path, params).await
    }

    pub async fn user_live_status(&self, subs: Vec<Subscription>) -> Vec<BilibiliLive> {
        let mut lives = vec![];
        for room_id in subs.iter().map(|sub| sub.user.id.parse::<u64>().unwrap()) {
            if let Some(result) = self.get_info_by_room(room_id).await {
                let info = result["data"]["room_info"].as_object().unwrap();
                let id = info["room_id"].as_u64().unwrap();
                lives.push(BilibiliLive {
                    id,
                    url: format!("https://live.bilibili.com/{id}").parse().unwrap(),
                    title: info["title"].as_str().unwrap().to_owned(),
                    creator_name: info["anchor_info"]["base_info"]["uname"].as_str().unwrap().to_owned(),
                    creator_id: info["uid"].as_u64().unwrap(),
                    cover_image_url: info["cover"].as_str().unwrap().parse().unwrap(),
                    start_time: DateTime::from_timestamp(info["live_start_time"].as_i64().unwrap(), 0).unwrap(),
                    state: match info["live_status"].as_u64().unwrap() {
                        0 => LiveState::Ended,
                        1 => LiveState::Running,
                        status => LiveState::Unknown(status.to_string())
                    }
                });
            }
        }
        lives
    }
}

impl fmt::Display for BilibiliLive {
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
