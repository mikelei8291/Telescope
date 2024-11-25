use core::fmt;
use std::str::FromStr;

use teloxide::utils::{command::ParseError, markdown::{bold, escape}};
use url::Url;

use crate::platform::{Platform, User};

#[derive(Clone)]
pub struct Subscription {
    pub platform: Platform,
    pub user: User
}

impl fmt::Display for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.platform, bold(escape(self.user.username.as_str()).as_str()))
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
        let (Some(&platform_str), Some(&user_id), Some(&username)) = (split.get(0), split.get(1), split.get(2)) else {
            return Err(SubscriptionError::InvalidFormat);
        };
        let Ok(platform) = Platform::from_str(platform_str) else {
            return Err(SubscriptionError::UnsupportedPlatform);
        };
        let user = User { user_id: user_id.to_string(), username: username.to_string() };
        Ok(Subscription { platform, user })
    }
}

impl Subscription {
    async fn from_host_and_path(host: &str, path: &str) -> Result<Self, ParseError> {
        if let Ok(platform) = host.parse::<Platform>() {
            if let Some(user) = platform.parse_user(path).await {
                return Ok(Subscription { platform, user });
            } else {
                return Err(ParseError::IncorrectFormat("Invalid username".into()));
            }
        } else {
            return Err(ParseError::Custom(format!("Unsupported platform: {host}").into()));
        };
    }

    pub async fn from_url(mut input: String) -> Result<Self, ParseError> {
        if !input.starts_with("http://") || !input.starts_with("https://") {
            input = format!("https://{input}");
        }
        match Url::parse(&input) {
            Ok(url) => {
                match url.host() {
                    Some(host) => Ok(Self::from_host_and_path(host.to_string().as_str(), url.path()).await?),
                    None => Err(ParseError::IncorrectFormat("Hostname not found".into()))
                }
            }
            Err(err) => Err(ParseError::IncorrectFormat(err.into()))
        }
    }

    pub fn to_db_string(&self) -> String {
        format!("{}:{}:{}", self.platform, self.user.user_id, self.user.username)
    }
}
