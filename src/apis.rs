use std::{env, fmt::Display, sync::Arc};

use bilibili::BilibiliAPI;
use redis::ToRedisArgs;
use reqwest::{cookie::Jar, header::HeaderMap, Client};
use serde::Serialize;
use serde_json::Value;
use strum_macros::EnumString;
use tokio::sync::OnceCell;
use twitter::TwitterAPI;
use url::Url;

use crate::subscription::Subscription;

pub mod twitter;
pub mod bilibili;

#[derive(EnumString)]
pub enum LiveState {
    Running,
    Ended,
    TimedOut,
    #[strum(default)]
    Unknown(String)
}

pub trait Metadata {
    type Id: ToRedisArgs;

    fn get_id(&self) -> &Self::Id;
    fn get_state(&self) -> &LiveState;
    fn to_sub(&self) -> Subscription;
}

pub trait API<T: Metadata + Display> {
    async fn live_status(&self, live_id: &String, language: Option<String>) -> Option<T>;
    async fn user_live_status(&self, subs: Vec<Subscription>) -> Vec<T>;
}

pub struct APIClient {
    base_url: Url,
    client: Client
}

impl APIClient {
    pub fn new(base_url: Url, headers: HeaderMap, cookies: Option<Jar>) -> Self {
        let mut cb = Client::builder().default_headers(headers);
        if let Some(cookies) = cookies {
            cb = cb.cookie_provider(cookies.into())
        }
        let client = cb.build().unwrap();
        Self { base_url, client }
    }

    pub async fn get<T: Serialize>(&self, path: &[&str], params: Option<T>) -> Option<Value> {
        let url = self.base_url.join(&path.join("/")).unwrap();
        let mut cb = self.client.get(url.clone());
        if let Some(params) = params {
            cb = cb.query(&params);
        }
        let Ok(res) = cb.send().await else {
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
}

pub static TWITTER_API: OnceCell<Arc<TwitterAPI>> = OnceCell::const_new();
pub static BILIBILI_API: OnceCell<Arc<BilibiliAPI>> = OnceCell::const_new();

pub async fn get_twitter_api() -> Arc<TwitterAPI> {
    TWITTER_API.get_or_init(|| async {
        Arc::new(TwitterAPI::new(
            &env::var("TWITTER_AUTH_TOKEN").unwrap(),
            &env::var("TWITTER_CSRF_TOKEN").unwrap()
        ))
    }).await.to_owned()
}

pub async fn get_bilibili_api() -> Arc<BilibiliAPI> {
    BILIBILI_API.get_or_init(|| async { Arc::new(BilibiliAPI::new()) }).await.to_owned()
}
