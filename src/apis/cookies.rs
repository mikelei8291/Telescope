use std::{collections::HashMap, str::from_utf8, sync::RwLock};

use cookie::{Cookie, ParseError};
use reqwest::{cookie::CookieStore, header::HeaderValue};
use url::Url;

#[derive(Default)]
pub struct SimpleCookieJar(RwLock<HashMap<String, String>>);

impl SimpleCookieJar {
    pub fn add_cookie(&self, name: &str, value: &str) {
        self.0.write().unwrap().insert(name.to_owned(), value.to_owned());
    }
}

impl CookieStore for SimpleCookieJar {
    fn set_cookies(&self, cookie_headers: &mut dyn Iterator<Item = &HeaderValue>, _: &Url) {
        let iter = cookie_headers.filter_map(
            |v| from_utf8(v.as_bytes()).map_err(ParseError::from).and_then(Cookie::parse).ok()
        ).map(|c| (c.name().to_owned(), c.value().to_owned()));
        self.0.write().unwrap().extend(iter);
    }

    fn cookies(&self, _: &Url) -> Option<HeaderValue> {
        let s = self.0.read().unwrap().iter().map(|(name, value)| format!("{name}={value}")).collect::<Vec<_>>().join("; ");
        if s.is_empty() {
            return None;
        }
        HeaderValue::from_str(&s).ok()
    }
}
