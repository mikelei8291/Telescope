use std::{env, fmt::Display, sync::Arc};

use bilibili::BilibiliAPI;
use redis::ToRedisArgs;
use strum_macros::EnumString;
use tokio::sync::OnceCell;
use twitter::TwitterAPI;

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
