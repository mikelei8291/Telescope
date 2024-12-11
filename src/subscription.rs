use std::{fmt::Display, str::FromStr};

use lazy_static::lazy_static;
use redis::{ErrorKind, FromRedisValue, RedisError, ToRedisArgs};
use regex::Regex;
use strum_macros::Display;
use teloxide::utils::{command::ParseError, markdown::{bold, escape}};
use url::Url;

use crate::platform::{Platform, User};

lazy_static! {
    static ref URL_REGEX: Regex = Regex::new(r"^https?://.+$").unwrap();
}

#[derive(Clone)]
pub struct Subscription {
    pub platform: Platform,
    pub user: User
}

impl Display for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.platform, bold(escape(self.user.username.as_str()).as_str()))
    }
}

impl ToRedisArgs for Subscription {
    fn write_redis_args<W>(&self, out: &mut W)
        where
            W: ?Sized + redis::RedisWrite {
        out.write_arg(self.to_db_string().as_bytes());
    }
}

impl FromRedisValue for Subscription {
    fn from_redis_value(v: &redis::Value) -> redis::RedisResult<Self> {
        String::from_redis_value(v)
            .map(|s| s.parse().map_err(
                |e: <Subscription as FromStr>::Err| RedisError::from(
                    (ErrorKind::TypeError, "Invalid database value", e.to_string())
                )
            ))?
    }
}

#[derive(Debug, Display)]
pub enum SubscriptionError {
    UnsupportedPlatform,
    InvalidFormat
}

impl FromStr for Subscription {
    type Err = SubscriptionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let split: Vec<&str> = s.split(":").collect();
        let (Some(&platform_str), Some(&id), Some(&username)) = (split.get(0), split.get(1), split.get(2)) else {
            return Err(SubscriptionError::InvalidFormat);
        };
        let Ok(platform) = Platform::from_str(platform_str) else {
            return Err(SubscriptionError::UnsupportedPlatform);
        };
        let user = User { id: id.to_owned(), username: username.to_owned() };
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
        if !URL_REGEX.is_match(&input) {
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
        format!("{}:{}:{}", self.platform, self.user.id, self.user.username)
    }
}
