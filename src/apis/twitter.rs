use core::fmt;
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use reqwest::{cookie::Jar, header::{self, HeaderMap, HeaderValue}, Client};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use teloxide::utils::markdown::{bold, code_block_with_lang, escape, link};
use url::Url;

use crate::{platform::{Platform, User}, subscription::Subscription};

use super::{LiveState, Metadata, API};

#[derive(Debug)]
pub struct TwitterAPI {
    client: Client,
    graph_ql_api: Url,
    fleets_api: Url,
    live_video_stream_api: Url
}

pub struct TwitterSpace {
    pub id: String,
    pub url: Url,
    pub title: String,
    pub creator_name: String,
    pub creator_id: String,
    pub creator_screen_name: String,
    pub creator_profile_image_url: Url,
    pub start_time: DateTime<Utc>,
    pub state: LiveState,
    pub language: String,
    pub available_for_replay: bool,
    pub master_url: Option<Url>
}

impl Metadata for TwitterSpace {
    type Id = String;

    fn get_id(&self) -> &Self::Id {
        &self.id
    }

    fn get_state(&self) -> &LiveState {
        &self.state
    }

    fn to_sub(&self) -> Subscription {
        Subscription {
            platform: Platform::TwitterSpace,
            user: User { id: self.creator_id.clone(), username: self.creator_screen_name.clone() }
        }
    }
}

enum Endpoint {
    GraphQL,
    Fleets,
    LiveVideoStream
}

#[derive(Serialize, Deserialize)]
struct ProfileSpotlightsQueryVariables {
    screen_name: String
}

#[derive(Serialize, Deserialize)]
struct AudioSpaceByIdVariables {
    id: String,
    #[serde(rename = "isMetatagsQuery")]
    is_metatags_query: bool,
    #[serde(rename = "withReplays")]
    with_replays: bool,
    #[serde(rename = "withListeners")]
    with_listeners: bool
}

impl TwitterAPI {
    pub fn new(auth_token: &str, csrf_token: &str) -> Self {
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
        Self {
            client,
            graph_ql_api: base_url.join("graphql/").unwrap(),
            fleets_api: base_url.join("fleets/").unwrap(),
            live_video_stream_api: base_url.join("1.1/live_video_stream/").unwrap()
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self, endpoint: Endpoint, path: String, params: Option<HashMap<String, String>>
    ) -> Option<T> {
        let url = match endpoint {
            Endpoint::GraphQL => self.graph_ql_api.join(path.as_str()).unwrap(),
            Endpoint::Fleets => self.fleets_api.join(path.as_str()).unwrap(),
            Endpoint::LiveVideoStream => self.live_video_stream_api.join(path.as_str()).unwrap()
        };
        let mut cb = self.client.get(url.clone());
        if let Some(params) = params {
            cb = cb.query(&params);
        }
        let Ok(res) = cb.send().await else {
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
        log::error!("{}: {}: {:?}", url, res.status(), res);
        None
    }

    async fn audio_space_by_id(&self, space_id: String) -> Option<Value> {
        let query_id = "xVEzTKg_mLTHubK5ayL0HA";
        let operation_name = "AudioSpaceById";
        let variables = AudioSpaceByIdVariables {
            id: space_id,
            is_metatags_query: true,
            with_replays: true,
            with_listeners: true
        };
        let features = "{\"spaces_2022_h2_clipping\":true,\"spaces_2022_h2_spaces_communities\":true,\"responsive_web_graphql_exclude_directive_enabled\":true,\"verified_phone_label_enabled\":false,\"creator_subscriptions_tweet_preview_api_enabled\":true,\"responsive_web_graphql_skip_user_profile_image_extensions_enabled\":false,\"tweetypie_unmention_optimization_enabled\":true,\"responsive_web_edit_tweet_api_enabled\":true,\"graphql_is_translatable_rweb_tweet_is_translatable_enabled\":true,\"view_counts_everywhere_api_enabled\":true,\"longform_notetweets_consumption_enabled\":true,\"responsive_web_twitter_article_tweet_consumption_enabled\":false,\"tweet_awards_web_tipping_enabled\":false,\"freedom_of_speech_not_reach_fetch_enabled\":true,\"standardized_nudges_misinfo\":true,\"tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled\":true,\"responsive_web_graphql_timeline_navigation_enabled\":true,\"longform_notetweets_rich_text_read_enabled\":true,\"longform_notetweets_inline_media_enabled\":true,\"responsive_web_media_download_video_enabled\":false,\"responsive_web_enhance_cards_enabled\":false}";
        let mut params = HashMap::new();
        params.insert("variables".to_owned(), serde_json::to_string(&variables).unwrap());
        params.insert("features".to_owned(), features.to_owned());
        self.get(Endpoint::GraphQL, [query_id, operation_name].join("/"), Some(params)).await
    }

    async fn profile_spotlights_query(&self, screen_name: String) -> Option<Value> {
        let query_id = "ZQEuHPrIYlvh1NAyIQHP_w";
        let operation_name = "ProfileSpotlightsQuery";
        let variables = ProfileSpotlightsQueryVariables { screen_name };
        let mut params = HashMap::new();
        params.insert("variables".to_owned(), serde_json::to_string(&variables).unwrap());
        self.get(Endpoint::GraphQL, [query_id, operation_name].join("/"), Some(params)).await
    }

    async fn avatar_content(&self, user_ids: &[String]) -> Option<Value> {
        let version = "v1";
        let endpoint = "avatar_content";
        let mut params = HashMap::new();
        params.insert("user_ids".to_owned(), user_ids.join(","));
        params.insert("only_spaces".to_owned(), "true".to_owned());
        self.get(Endpoint::Fleets, [version, endpoint].join("/"), Some(params)).await
    }

    async fn status(&self, media_key: &str) -> Option<Value> {
        self.get(Endpoint::LiveVideoStream, ["status", media_key].join("/"), None).await
    }

    pub async fn user_id(&self, screen_name: &String) -> Option<String> {
        if let Some(result) = self.profile_spotlights_query(screen_name.to_owned()).await {
            let value = &result["data"]["user_result_by_screen_name"]["result"]["rest_id"];
            Some(value.as_str()?.to_string())
        } else {
            None
        }
    }
}

impl API<TwitterSpace> for TwitterAPI {
    async fn live_status(&self, live_id: &String, language: Option<String>) -> Option<TwitterSpace> {
        if let Some(space) = self.audio_space_by_id(live_id.clone()).await {
            let metadata = space["data"]["audioSpace"]["metadata"].as_object().unwrap();
            let state = metadata["state"].as_str().unwrap().parse().unwrap_or(LiveState::Ended);
            let master_url = match state {
                LiveState::Running => {
                    if let Some(live_status) = self.status(metadata["media_key"].as_str().unwrap()).await {
                        Some(live_status["source"]["location"].as_str().unwrap()
                            .replace("dynamic_playlist.m3u8?type=live", "master_playlist.m3u8").parse().unwrap())
                    } else {
                        None
                    }
                }
                _ => None
            };
            Some(TwitterSpace {
                id: live_id.clone(),
                url: format!("https://twitter.com/i/spaces/{live_id}").parse().unwrap(),
                title: metadata["title"].as_str().unwrap().to_owned(),
                creator_name: metadata["creator_results"]["result"]["legacy"]["name"].as_str().unwrap().to_owned(),
                creator_id: metadata["creator_results"]["result"]["rest_id"].as_str().unwrap().to_owned(),
                creator_screen_name: metadata["creator_results"]["result"]["legacy"]["screen_name"].as_str().unwrap().to_owned(),
                creator_profile_image_url: metadata["creator_results"]["result"]["legacy"]["profile_image_url_https"].as_str().unwrap().parse().unwrap(),
                start_time: DateTime::from_timestamp_millis(metadata["started_at"].as_i64().unwrap()).unwrap(),
                state,
                language: language.unwrap_or("und".to_owned()),
                available_for_replay: metadata["is_space_available_for_replay"].as_bool().unwrap(),
                master_url
            })
        } else {
            None
        }
    }

    async fn user_live_status(&self, subs: Vec<Subscription>) -> Vec<TwitterSpace> {
        let mut spaces = vec![];
        for user_ids in subs.iter().map(|sub| sub.user.id.clone()).collect::<Vec<String>>().chunks(100) {
            if let Some(result) = self.avatar_content(user_ids).await {
                for value in result["users"].as_object().unwrap().values() {
                    let audio_space = &value["spaces"]["live_content"]["audiospace"];
                    if let Some(space) = self.live_status(
                        &audio_space["broadcast_id"].as_str().unwrap().to_owned(),
                        Some(audio_space["language"].as_str().unwrap().to_owned())
                    ).await {
                        match space.state {
                            LiveState::Running => spaces.push(space),
                            _ => ()
                        }
                    }
                }
            }
        }
        spaces
    }
}

impl fmt::Display for TwitterSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.state {
            LiveState::Running => write!(
                f,
                "{} \\({}\\)'s Twitter Space started\n{}\n{}",
                bold(escape(self.creator_name.as_str()).as_str()),
                link(
                    format!("https://twitter.com/{}", self.creator_screen_name).as_str(),
                    format!("@{}", self.creator_screen_name).as_str()
                ),
                link(self.url.as_str(), escape(self.title.as_str()).as_str()),
                code_block_with_lang(format!("twspace_dl -ei {} -f {}", self.url, self.master_url.clone().unwrap()).as_str(), "shell")
            ),
            LiveState::Ended | LiveState::TimedOut => write!(
                f,
                "{} \\({}\\)'s Twitter Space ended",
                bold(escape(self.creator_name.as_str()).as_str()),
                link(
                    format!("https://twitter.com/{}", self.creator_screen_name).as_str(),
                    format!("@{}", self.creator_screen_name).as_str()
                )
            ),
            LiveState::Unknown(state) => f.write_str(escape(format!("Unknown live state: {state}").as_str()).as_str())
        }
    }
}
