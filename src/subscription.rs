use core::fmt;
use std::str::FromStr;

use lazy_static::lazy_static;
use regex::{Captures, Regex};
use teloxide::utils::command::ParseError;
use url::Url;
use strum_macros::{Display, EnumString, EnumIter};

#[derive(Clone, Display, EnumString, EnumIter)]
pub enum Platform {
    #[strum(to_string = "Twitter Space")]
    TwitterSpace
}

impl Platform {
    pub fn parse_user(self: &Self, path: &str) -> Option<String> {
        lazy_static! {
            static ref TWITTER_USERNAME: Regex = Regex::new(r"^/(?P<username>\w{4,15})/?$").unwrap();
        }
        let get_group = |m: Captures| Some(m["username"].to_string());
        match self {
            Platform::TwitterSpace => TWITTER_USERNAME.captures(path).and_then(get_group)
        }
    }
}

#[derive(Clone)]
pub struct Subscription {
    pub platform: Platform,
    pub user_id: String
}

impl fmt::Display for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.platform, self.user_id)
    }
}

pub enum SubscriptionError {
    UnsupportedPlatform,
    InvalidFormat
}

impl FromStr for Subscription {
    type Err = SubscriptionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = s.split(":").collect();
        let (Some(&platform_str), Some(&user_id)) = (split.get(0), split.get(1)) else {
            return Err(SubscriptionError::InvalidFormat);
        };
        let Ok(platform) = Platform::from_str(platform_str) else {
            return Err(SubscriptionError::UnsupportedPlatform);
        };
        Ok(Subscription { platform, user_id: user_id.to_string() })
    }
}

fn parse_subscription(host: &str, path: &str) -> Result<Subscription, ParseError> {
    if let Some(platform) = match host {
        "twitter.com" | "x.com" => Some(Platform::TwitterSpace),
        _ => None
    } {
        if let Some(user_id) = platform.parse_user(path) {
            return Ok(Subscription { platform, user_id });
        } else {
            return Err(ParseError::IncorrectFormat("Invalid username".into()));
        }
    } else {
        return Err(ParseError::Custom(format!("Unsupported platform: {host}").into()));
    };
}

pub fn parse_url(mut input: String) -> Result<(Subscription,), ParseError> {
    if !input.starts_with("http://") || !input.starts_with("https://") {
        input = format!("https://{input}");
    }
    match Url::parse(&input) {
        Ok(url) => {
            match url.host() {
                Some(host) => Ok((parse_subscription(host.to_string().as_str(), url.path())?,)),
                None => Err(ParseError::IncorrectFormat("Hostname not found".into()))
            }
        }
        Err(err) => Err(ParseError::IncorrectFormat(err.into()))
    }
}