use core::fmt;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use reqwest::{cookie::Jar, header::{self, HeaderMap, HeaderValue}, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use teloxide::utils::markdown::{code_block_with_lang, escape, link};
use url::Url;

#[derive(Debug)]
pub struct API {
    pub client: Client,
    graph_ql_api: Url,
    fleets_api: Url
}

pub struct TwitterSpace {
    pub id: String,
    pub url: Url,
    pub title: String,
    pub creator_name: Option<String>,
    pub creator_id: String,
    pub creator_screen_name: String,
    pub creator_profile_image_url: Option<Url>,
    pub start_time: DateTime<Utc>,
    pub state: String,
    pub language: String,
    pub available_for_replay: bool,
    pub media_key: Option<String>
}

enum Endpoint {
    GraphQL,
    Fleets
}

#[derive(Serialize, Deserialize)]
struct ProfileSpotlightsQueryVariables {
    screen_name: String
}

impl API {
    pub fn new(auth_token: &str, csrf_token: &str) -> API {
        let base_url: Url = "https://x.com/i/api/".parse().unwrap();
        let mut headers = HeaderMap::new();
        headers.append(
            header::AUTHORIZATION,
            HeaderValue::from_static(
                "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA"
            )
        );
        headers.append("x-csrf-token", HeaderValue::from_str(csrf_token).unwrap());
        let cookies = Jar::default();
        cookies.add_cookie_str(format!("auth_token={auth_token}; Domain={}", base_url.host_str().unwrap()).as_str(), &base_url);
        cookies.add_cookie_str(format!("ct0={csrf_token}; Domain={}", base_url.host_str().unwrap()).as_str(), &base_url);
        let client = Client::builder()
            .default_headers(headers)
            .cookie_provider(cookies.into())
            .build().unwrap();
        API {
            client,
            graph_ql_api: base_url.join("graphql/").unwrap(),
            fleets_api: base_url.join("fleets/").unwrap()
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self, endpoint: Endpoint, path: String, params: HashMap<String, String>
    ) -> Option<T> {
        let url = match endpoint {
            Endpoint::GraphQL => self.graph_ql_api.join(path.as_str()).unwrap(),
            Endpoint::Fleets => self.fleets_api.join(path.as_str()).unwrap()
        };
        let Ok(res) = self.client.get(url).form(&params).send().await else {
            log::error!("API error");
            return None;
        };
        if res.status().is_success() {
            let Ok(data) = res.json::<T>().await else {
                log::error!("JSON decode error");
                return None;
            };
            return Some(data);
        }
        log::error!("{}: {}", res.status(), res.text().await.unwrap());
        None
    }

    async fn profile_spotlights_query(&self, screen_name: String) -> Option<Value> {
        let query_id = "ZQEuHPrIYlvh1NAyIQHP_w";
        let operation_name = "ProfileSpotlightsQuery";
        let variables = ProfileSpotlightsQueryVariables { screen_name };
        let mut params = HashMap::new();
        params.insert("variables".to_owned(), serde_json::to_string(&variables).unwrap());
        self.get(Endpoint::GraphQL, [query_id, operation_name].join("/"), params).await
    }

    async fn avatar_content(&self, user_ids: Vec<String>) -> Option<Value> {
        let version = "v1";
        let endpoint = "avatar_content";
        let mut params = HashMap::new();
        params.insert("user_ids".to_owned(), user_ids.join(","));
        params.insert("only_spaces".to_owned(), "true".to_owned());
        self.get(Endpoint::Fleets, [version, endpoint].join("/"), params).await
    }

    pub async fn user_id(&self, screen_name: String) -> Option<String> {
        if let Some(result) = self.profile_spotlights_query(screen_name.clone()).await {
            let value = &result["data"]["user_result_by_screen_name"]["result"]["rest_id"];
            Some(value.as_str()?.to_string())
        } else {
            None
        }
    }

    pub async fn live_status(&self, user_map: HashMap<String, String>) -> HashMap<String, TwitterSpace> {
        let mut users = HashMap::new();
        for chunk in user_map.keys().collect::<Vec<&String>>().chunks(100) {
            let user_ids = chunk.iter().map(|&p| p.to_owned()).collect();
            if let Some(result) = self.avatar_content(user_ids).await {
                users.extend(
                    result["users"].as_object().unwrap().iter().map(
                        |(key, value)| {
                            let audio_space = &value["spaces"]["live_content"]["audiospace"];
                            let id = audio_space["broadcast_id"].as_str().unwrap().to_owned();
                            (key.to_owned(), TwitterSpace {
                                id: id.clone(),
                                url: format!("https://twitter.com/i/spaces/{id}").parse().unwrap(),
                                title: audio_space["title"].as_str().unwrap().to_owned(),
                                creator_name: None,
                                creator_id: audio_space["creator_twitter_user_id"].as_u64().unwrap().to_string(),
                                creator_screen_name: user_map[&id].to_owned(),
                                creator_profile_image_url: None,
                                start_time: audio_space["start"].as_str().unwrap().parse().unwrap(),
                                state: "Running".to_owned(),
                                language: audio_space["language"].as_str().unwrap().to_owned(),
                                available_for_replay: audio_space["is_space_available_for_replay"].as_bool().unwrap(),
                                media_key: None
                            })
                        }
                    )
                );
            }
        }
        users
    }
}

impl fmt::Display for TwitterSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}'s Twitter Space started\n{}\n{}",
            link(
                format!("https://twitter.com/{}", self.creator_screen_name).as_str(),
                format!("@{}", self.creator_screen_name).as_str()
            ),
            link(self.url.as_str(), escape(self.title.as_str()).as_str()),
            code_block_with_lang(format!("twspace_dl -ei {}", self.url.to_string()).as_str(), "shell")
        )
    }
}
