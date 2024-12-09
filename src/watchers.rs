use core::fmt;
use std::{sync::Arc, time::Duration};

use redis::{aio::MultiplexedConnection, AsyncCommands, AsyncIter};
use teloxide::{payloads::SendMessageSetters, prelude::Requester, types::{LinkPreviewOptions, MessageId, ReplyParameters}};
use tokio::{task, time};

use crate::{apis::{get_bilibili_api, get_twitter_api, LiveState, Metadata, API}, platform::Platform, subscription::Subscription, Bot};

pub async fn check<T: Metadata + fmt::Display>(api: Arc<impl API<T>>, db: &mut MultiplexedConnection, bot: &Bot, platform: Platform) {
    let mut subs: Vec<Subscription> = vec![];
    let mut db_clone = db.clone();
    let mut iter: AsyncIter<(String, String)> = db_clone.hscan_match("subs", format!("{}:*", platform)).await.unwrap();
    while let Some((sub_str, live_id)) = iter.next_item().await {
        if live_id.as_str() == "" {
            if let Ok(sub) = sub_str.parse::<Subscription>() {
                subs.push(sub);
            } else {
                log::error!("Database error: invalid sub string: {}", sub_str);
            };
        } else {
            if let Some(live) = api.live_status(&live_id, None).await {
                let mut db_clone = db.clone();
                let mut iter: AsyncIter<(String, i32)> = db_clone.hscan(&sub_str).await.unwrap();
                match live.get_state() {
                    LiveState::Running => (),
                    LiveState::Ended | LiveState::TimedOut => {
                        while let Some((chat_id, msg_id)) = iter.next_item().await {
                            bot.send_message(chat_id.clone(), live.to_string()).link_preview_options(LinkPreviewOptions {
                                    is_disabled: true,
                                    url: Default::default(),
                                    prefer_small_media: Default::default(),
                                    prefer_large_media: Default::default(),
                                    show_above_text: Default::default()
                                }).reply_parameters(ReplyParameters::new(MessageId(msg_id))).await.unwrap();
                            redis::pipe().atomic()
                                .hset("subs", &sub_str, "")
                                .hset(&sub_str, chat_id, 0)
                                .exec_async(db).await.unwrap();
                        }
                    }
                    LiveState::Unknown(_) => {
                        while let Some((chat_id, msg_id)) = iter.next_item().await {
                            bot.send_message(chat_id, live.to_string()).reply_parameters(ReplyParameters::new(MessageId(msg_id))).await.unwrap();
                        }
                    }
                }
            }
        }
    }
    for live in api.user_live_status(subs).await {
        let sub = live.to_sub();
        let subscribers: Vec<String> = db.hkeys(sub.to_db_string()).await.unwrap();
        for chat_id in subscribers {
            let msg = bot.send_message(chat_id.clone(), live.to_string()).link_preview_options(LinkPreviewOptions {
                is_disabled: true,
                url: Default::default(),
                prefer_small_media: Default::default(),
                prefer_large_media: Default::default(),
                show_above_text: Default::default()
            }).await.unwrap();
            redis::pipe().atomic()
                .hset("subs", sub.to_db_string(), live.get_id())
                .hset(sub.to_db_string(), chat_id, msg.id.0)
                .exec_async(db).await.unwrap();
        }
    }
}

pub fn watch(mut db: MultiplexedConnection, bot: Bot) -> task::JoinHandle<()> {
    task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        loop {
            check(get_twitter_api().await, &mut db, &bot, Platform::TwitterSpace).await;
            check(get_bilibili_api().await, &mut db, &bot, Platform::BilibiliLive).await;
            interval.tick().await;
        }
    })
}
