use regex::{Captures, Regex};
use teloxide::utils::command::ParseError;
use url::Url;
use lazy_static::lazy_static;

#[derive(Clone, strum_macros::Display)]
pub enum Platform {
    #[strum(to_string = "Twitter Space")]
    TwitterSpace
}

#[derive(Clone)]
pub struct Subscription {
    pub platform: Platform,
    pub user_id: String
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

pub fn parse_url(input: String) -> Result<(Subscription,), ParseError> {
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