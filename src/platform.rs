use lazy_static::lazy_static;
use regex::{Captures, Regex};
use strum_macros::{Display, EnumIter, EnumString};

use crate::apis::{get_bilibili_api, get_twitter_api};

#[derive(Clone)]
pub struct User {
    pub id: String,
    pub username: String
}

#[derive(Clone, Display, EnumString, EnumIter)]
pub enum Platform {
    #[strum(to_string = "Twitter Space", serialize = "twitter.com", serialize = "x.com")]
    TwitterSpace,
    #[strum(to_string = "Bilibili Live", serialize = "live.bilibili.com")]
    BilibiliLive
}

impl Platform {
    pub async fn parse_user(self: &Self, path: &str) -> Option<User> {
        lazy_static! {
            static ref TWITTER_USERNAME: Regex = Regex::new(r"^/(?P<username>\w{4,15})/?$").unwrap();
            static ref BILIBILI_ROOM_ID: Regex = Regex::new(r"^/(?P<room_id>\d+)/?$").unwrap();
        }
        match self {
            Platform::TwitterSpace => {
                let Some(username) = TWITTER_USERNAME.captures(path).and_then(
                    |m: Captures| Some(m["username"].to_owned())
                ) else {
                    return None;
                };
                let api = get_twitter_api().await;
                let Some(id) = api.user_id(username.clone()).await else {
                    return None;
                };
                Some(User { id, username })
            }
            Platform::BilibiliLive => {
                let Some(id) = BILIBILI_ROOM_ID.captures(path).and_then(
                    |m: Captures| Some(m["room_id"].to_owned())
                ) else {
                    return None;
                };
                let api = get_bilibili_api().await;
                let Some(username) = api.username(id.clone()).await else {
                    return None;
                };
                Some(User { id, username })
            }
        }
    }
}
