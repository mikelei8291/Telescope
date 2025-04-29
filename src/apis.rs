use std::{env, fmt::Display, sync::Arc};

use bilibili::BilibiliAPI;
use cookies::SimpleCookieJar;
use redis::ToRedisArgs;
use reqwest::{header::HeaderMap, Client};
use serde::Serialize;
use serde_json::Value;
use strum_macros::EnumString;
use teloxide::types::InputFile;
use tokio::sync::OnceCell;
use twitter::TwitterAPI;
use url::Url;

use crate::{log_utils::LogResult, subscription::Subscription};

mod cookies;
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
    fn get_attachment(&self) -> InputFile;
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
    pub fn new(base_url: &str, headers: HeaderMap, cookies: Option<SimpleCookieJar>) -> Self {
        let mut cb = Client::builder().default_headers(headers);
        if let Some(cookies) = cookies {
            cb = cb.cookie_provider(cookies.into());
        }
        let client = cb.build().expect("Failed to create API client");
        Self { base_url: base_url.parse().expect("Invalid base URL"), client }
    }

    pub async fn get<T: Serialize>(&self, path: &[&str], params: Option<T>) -> Option<Value> {
        let url = self.base_url.join(&path.join("/")).log_ok("Invalid request URL")?;
        let mut req = self.client.get(url.clone());
        if let Some(params) = params {
            req = req.query(&params);
        }
        let res = req.send().await.log_ok("API error")?;
        if res.status().is_success() {
            return res.json().await.log_ok("JSON decode error");
        }
        log::error!("{}: {}: {:?}", url, res.status(), res);
        None
    }
}

pub static TWITTER_API: OnceCell<Arc<TwitterAPI>> = OnceCell::const_new();
pub static BILIBILI_API: OnceCell<Arc<BilibiliAPI>> = OnceCell::const_new();
const ENV_ERROR_MSG: &str = "Failed to load token from environment variables";

pub async fn get_twitter_api() -> &'static Arc<TwitterAPI> {
    TWITTER_API.get_or_init(async || {
        TwitterAPI::new(
            &env::var("TWITTER_AUTH_TOKEN").expect(ENV_ERROR_MSG),
            &env::var("TWITTER_CSRF_TOKEN").expect(ENV_ERROR_MSG)
        ).into()
    }).await
}

pub async fn get_bilibili_api() -> &'static Arc<BilibiliAPI> {
    BILIBILI_API.get_or_init(async || { BilibiliAPI::new().into() }).await
}
