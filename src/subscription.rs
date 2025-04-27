use std::{fmt::Display, str::FromStr};

use lazy_regex::{lazy_regex, Lazy};
use redis::{ErrorKind, FromRedisValue, RedisError, ToRedisArgs};
use regex::Regex;
use strum_macros::Display;
use teloxide::utils::{command::ParseError, markdown::{bold, escape}};
use url::Url;

use crate::platform::{Platform, User};

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

pub fn fmt_subscriptions(subs: &Vec<Subscription>) -> String {
    subs.iter().enumerate().map(|(i, s)| format!("{}\\. {s}", i + 1)).collect::<Vec<_>>().join("\n")
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
        String::from_redis_value(v)?
            .parse::<Subscription>()
            .map_err(|e| RedisError::from((ErrorKind::TypeError, "Invalid database value", e.to_string())))
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
        let split: Vec<_> = s.split(":").collect();
        let [platform_str, id, username] = split[..] else {
            return Err(SubscriptionError::InvalidFormat);
        };
        let platform = platform_str.parse().or(Err(SubscriptionError::UnsupportedPlatform))?;
        Ok(Subscription { platform, user: User { id: id.to_owned(), username: username.to_owned() } })
    }
}

impl Subscription {
    const URL_REGEX: Lazy<Regex> = lazy_regex!(r"^https?://.+$");

    async fn from_host_and_path(host: &str, path: &str) -> Result<Self, ParseError> {
        let platform: Platform = host.parse().or(Err(ParseError::Custom(format!("Unsupported platform: {host}").into())))?;
        let user = platform.parse_user(path).await.ok_or(ParseError::IncorrectFormat("Invalid username".into()))?;
        Ok(Self { platform, user })
    }

    pub async fn from_url(mut input: String) -> Result<Self, ParseError> {
        if !Self::URL_REGEX.is_match(&input) {
            input = format!("https://{input}");
        }
        let url = input.parse::<Url>().map_err(|err| ParseError::IncorrectFormat(err.into()))?;
        let host = url.host_str().ok_or(ParseError::IncorrectFormat("Hostname not found".into()))?;
        Self::from_host_and_path(host, url.path()).await
    }

    pub fn to_db_string(&self) -> String {
        format!("{}:{}:{}", self.platform, self.user.id, self.user.username)
    }
}
