use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;
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
    const TWITTER_USERNAME: Lazy<Regex> = lazy_regex!(r"^/(?P<username>\w{4,15})/?$");
    const BILIBILI_ROOM_ID: Lazy<Regex> = lazy_regex!(r"^/(?P<room_id>\d+)/?$");

    pub async fn parse_user(self: &Self, path: &str) -> Option<User> {
        match self {
            Platform::TwitterSpace => {
                let username = Self::TWITTER_USERNAME.captures(path)?["username"].to_owned();
                let id = get_twitter_api().await.user_id(&username).await?;
                Some(User { id, username })
            }
            Platform::BilibiliLive => {
                let id = Self::BILIBILI_ROOM_ID.captures(path)?["room_id"].to_owned();
                let username = get_bilibili_api().await.username(&id).await?;
                Some(User { id, username })
            }
        }
    }
}
