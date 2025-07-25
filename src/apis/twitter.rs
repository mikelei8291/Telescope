use std::{collections::HashMap, fmt::Display};

use chrono::{DateTime, Utc};
use futures::{stream::{self}, StreamExt};
use reqwest::header::{self, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use strum_macros::{Display, EnumString};
use teloxide::{types::InputFile, utils::markdown::{bold, code_block_with_lang, escape, link}};
use url::Url;

use crate::{log_utils::LogResult, platform::{Platform, User}, subscription::Subscription};

use super::{cookies::SimpleCookieJar, APIClient, LiveState, Metadata, API};

pub struct TwitterAPI {
    client: APIClient
}

#[allow(dead_code)]
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
    pub master_url: Option<Url>,
    pub metadata: Value
}

impl Metadata for TwitterSpace {
    type Id = String;

    fn get_id(&self) -> &Self::Id {
        &self.id
    }

    fn get_state(&self) -> &LiveState {
        &self.state
    }

    fn get_attachment(&self) -> InputFile {
        InputFile::memory(self.metadata.to_string()).file_name(format!("{}.json", self.id))
    }

    fn to_sub(&self) -> Subscription {
        Subscription {
            platform: Platform::TwitterSpace,
            user: User { id: self.creator_id.clone(), username: self.creator_screen_name.clone() }
        }
    }
}

#[derive(Display, EnumString)]
enum Endpoint {
    #[strum(to_string = "graphql")]
    GraphQL,
    #[strum(to_string = "fleets")]
    Fleets,
    #[strum(to_string = "1.1/live_video_stream")]
    LiveVideoStream
}

#[derive(Serialize, Deserialize)]
struct ProfileSpotlightsQueryVariables<'a> {
    screen_name: &'a str
}

#[derive(Serialize, Deserialize)]
struct AudioSpaceByIdVariables<'a> {
    id: &'a str,
    #[serde(rename = "isMetatagsQuery")]
    is_metatags_query: bool,
    #[serde(rename = "withReplays")]
    with_replays: bool,
    #[serde(rename = "withListeners")]
    with_listeners: bool
}

impl TwitterAPI {
    pub fn new(auth_token: &str, csrf_token: &str) -> Self {
        let mut headers = HeaderMap::new();
        headers.append(
            header::AUTHORIZATION,
            HeaderValue::from_static(
                "Bearer AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA"
            )
        );
        headers.append("x-csrf-token", HeaderValue::from_str(csrf_token).expect("Invalid x-csrf-token"));
        let cookies = SimpleCookieJar::default();
        cookies.add_cookie("auth_token", auth_token);
        cookies.add_cookie("ct0", csrf_token);
        Self { client: APIClient::new("https://x.com/i/api/", headers, Some(cookies)) }
    }

    async fn audio_space_by_id(&self, space_id: &str) -> Option<Value> {
        let query_id = "xVEzTKg_mLTHubK5ayL0HA";
        let operation_name = "AudioSpaceById";
        let variables = AudioSpaceByIdVariables {
            id: space_id,
            is_metatags_query: true,
            with_replays: true,
            with_listeners: true
        };
        let features = "{\"spaces_2022_h2_clipping\":true,\"spaces_2022_h2_spaces_communities\":true,\"responsive_web_graphql_exclude_directive_enabled\":true,\"verified_phone_label_enabled\":false,\"creator_subscriptions_tweet_preview_api_enabled\":true,\"responsive_web_graphql_skip_user_profile_image_extensions_enabled\":false,\"tweetypie_unmention_optimization_enabled\":true,\"responsive_web_edit_tweet_api_enabled\":true,\"graphql_is_translatable_rweb_tweet_is_translatable_enabled\":true,\"view_counts_everywhere_api_enabled\":true,\"longform_notetweets_consumption_enabled\":true,\"responsive_web_twitter_article_tweet_consumption_enabled\":false,\"tweet_awards_web_tipping_enabled\":false,\"freedom_of_speech_not_reach_fetch_enabled\":true,\"standardized_nudges_misinfo\":true,\"tweet_with_visibility_results_prefer_gql_limited_actions_policy_enabled\":true,\"responsive_web_graphql_timeline_navigation_enabled\":true,\"longform_notetweets_rich_text_read_enabled\":true,\"longform_notetweets_inline_media_enabled\":true,\"responsive_web_media_download_video_enabled\":false,\"responsive_web_enhance_cards_enabled\":false}";
        let params = HashMap::from([
            ("variables", serde_json::to_string(&variables).ok()?),
            ("features", features.to_owned())
        ]);
        self.client.get(&[&Endpoint::GraphQL.to_string(), query_id, operation_name], Some(params)).await
    }

    async fn profile_spotlights_query(&self, screen_name: &str) -> Option<Value> {
        let query_id = "ZQEuHPrIYlvh1NAyIQHP_w";
        let operation_name = "ProfileSpotlightsQuery";
        let variables = ProfileSpotlightsQueryVariables { screen_name };
        let params = HashMap::from([
            ("variables", serde_json::to_string(&variables).ok()?)
        ]);
        self.client.get(&[&Endpoint::GraphQL.to_string(), query_id, operation_name], Some(params)).await
    }

    async fn avatar_content(&self, user_ids: &[&str]) -> Option<Value> {
        let version = "v1";
        let endpoint = "avatar_content";
        let params = HashMap::from([
            ("user_ids", user_ids.join(",")),
            ("only_spaces", true.to_string())
        ]);
        self.client.get(&[&Endpoint::Fleets.to_string(), version, endpoint], Some(params)).await
    }

    async fn status(&self, media_key: &str) -> Option<Value> {
        self.client.get::<()>(&[&Endpoint::LiveVideoStream.to_string(), "status", media_key], None).await
    }

    pub async fn user_id(&self, screen_name: &str) -> Option<String> {
        let result = self.profile_spotlights_query(screen_name).await?;
        let value = &result["data"]["user_result_by_screen_name"]["result"]["rest_id"];
        Some(value.as_str()?.to_string())
    }
}

impl API<TwitterSpace> for TwitterAPI {
    async fn live_status(&self, live_id: &String, language: Option<String>) -> Option<TwitterSpace> {
        let space = self.audio_space_by_id(live_id).await?;
        let metadata = space["data"]["audioSpace"]["metadata"].as_object()?;
        let state = metadata["state"].as_str()?.parse().ok()?;
        let master_url = if let LiveState::Running = state {
            let live_status = self.status(metadata["media_key"].as_str()?).await?;
            Some(live_status["source"]["location"].as_str()?
                .replace("dynamic_playlist.m3u8?type=live", "master_playlist.m3u8").parse().log_ok("Twitter Space Playlist URL")?)
        } else {
            None
        };
        Some(TwitterSpace {
            id: live_id.clone(),
            url: format!("https://twitter.com/i/spaces/{live_id}").parse().log_ok("Twitter Space URL")?,
            title: metadata["title"].as_str()?.to_owned(),
            creator_name: metadata["creator_results"]["result"]["legacy"]["name"].as_str()?.to_owned(),
            creator_id: metadata["creator_results"]["result"]["rest_id"].as_str()?.to_owned(),
            creator_screen_name: metadata["creator_results"]["result"]["legacy"]["screen_name"].as_str()?.to_owned(),
            creator_profile_image_url:
                metadata["creator_results"]["result"]["legacy"]["profile_image_url_https"].as_str()?.parse().log_ok("Twitter Profile Image URL")?,
            start_time: DateTime::from_timestamp_millis(metadata["started_at"].as_i64()?)?,
            state,
            language: language.unwrap_or("und".to_owned()),
            available_for_replay: metadata["is_space_available_for_replay"].as_bool()?,
            master_url,
            metadata: space
        })
    }

    async fn user_live_status(&self, subs: Vec<Subscription>) -> Vec<TwitterSpace> {
        let mut spaces = vec![];
        for user_ids in subs.iter().map(|sub| sub.user.id.as_str()).collect::<Vec<&str>>().chunks(100) {
            if let Some(result) = self.avatar_content(user_ids).await {
                if let Some(users) = result["users"].as_object().map(|o| stream::iter(o.values())) {
                    spaces.extend(
                        users.filter_map(async |value| {
                            let audio_space = &value["spaces"]["live_content"]["audiospace"].as_object()?;
                            self.live_status(
                                &audio_space.get("broadcast_id")?.as_str()?.to_owned(),
                                Some(audio_space.get("language")?.as_str()?.to_owned())
                            ).await.filter(|space| matches!(space.state, LiveState::Running))
                        }).collect::<Vec<_>>().await
                    );
                }
            }
        }
        spaces
    }
}

impl Display for TwitterSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.state {
            LiveState::Running => write!(
                f,
                "{} \\({}\\)'s Twitter Space started\n{}\n{}",
                bold(escape(self.creator_name.as_str()).as_str()),
                link(
                    format!("https://twitter.com/{}", escape(self.creator_screen_name.as_str())).as_str(),
                    format!("@{}", escape(self.creator_screen_name.as_str())).as_str()
                ),
                link(self.url.as_str(), escape(self.title.as_str()).as_str()),
                code_block_with_lang(
                    format!(
                        "twspace_dl -ei {}{}", self.url, self.master_url.as_ref().map(|s| format!(" -f {s}")).unwrap_or_default()
                    ).as_str(), "shell"
                )
            ),
            LiveState::Ended | LiveState::TimedOut => write!(
                f,
                "{} \\({}\\)'s Twitter Space ended",
                bold(escape(self.creator_name.as_str()).as_str()),
                link(
                    format!("https://twitter.com/{}", escape(self.creator_screen_name.as_str())).as_str(),
                    format!("@{}", escape(self.creator_screen_name.as_str())).as_str()
                )
            ),
            LiveState::Unknown(state) => f.write_str(escape(format!("Unknown live state: {state}").as_str()).as_str())
        }
    }
}
