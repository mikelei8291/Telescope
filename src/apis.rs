use std::{env, sync::Arc};

use strum_macros::EnumString;
use tokio::sync::OnceCell;

pub mod twitter;

#[derive(EnumString)]
pub enum LiveState {
    Running,
    Ended,
    TimedOut,
    #[strum(default)]
    Unknown(String)
}

pub static TWITTER_API: OnceCell<Arc<twitter::API>> = OnceCell::const_new();

pub fn get_twitter_api() -> Arc<twitter::API> {
    match TWITTER_API.get() {
        Some(api) => api.clone(),
        None => {
            let api = twitter::API::new(
                &env::var("TWITTER_AUTH_TOKEN").unwrap(),
                &env::var("TWITTER_CSRF_TOKEN").unwrap()
            );
            TWITTER_API.set(Arc::new(api)).unwrap();
            TWITTER_API.get().unwrap().clone()
        }
    }
}
