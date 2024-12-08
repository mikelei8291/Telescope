use std::{env, sync::Arc};

use strum_macros::EnumString;
use tokio::sync::OnceCell;

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

pub static TWITTER_API: OnceCell<Arc<twitter::API>> = OnceCell::const_new();

pub async fn get_twitter_api() -> Arc<twitter::API> {
    TWITTER_API.get_or_init(|| async {
        Arc::new(twitter::API::new(
            &env::var("TWITTER_AUTH_TOKEN").unwrap(),
            &env::var("TWITTER_CSRF_TOKEN").unwrap()
        ))
    }).await.to_owned()
}
