use std::{fmt::Display, sync::Arc, time::Duration};

use redis::{aio::MultiplexedConnection, AsyncCommands, AsyncIter};
use teloxide::{
    payloads::{SendDocumentSetters, SendPhotoSetters},
    prelude::Requester,
    sugar::request::{RequestLinkPreviewExt, RequestReplyExt},
    types::MessageId
};
use tokio::{task, time};

use crate::{
    apis::{get_bilibili_api, get_twitter_api, LiveState, Metadata, API},
    platform::Platform,
    subscription::Subscription,
    Bot
};

pub async fn check<T: Metadata + Display>(api: &Arc<impl API<T>>, db: &mut MultiplexedConnection, bot: &Bot, platform: Platform) {
    let mut subs: Vec<Subscription> = vec![];
    let mut db_clone = db.clone();
    let Ok(mut iter): Result<AsyncIter<(Subscription, String)>, _> = db_clone.hscan_match("subs", format!("{platform}:*")).await else {
        return;
    };
    while let Some((sub, live_id)) = iter.next_item().await {
        if live_id.is_empty() {
            subs.push(sub);
        } else {
            if let Some(live) = api.live_status(&live_id, None).await {
                let mut db_clone = db.clone();
                let mut iter: AsyncIter<(String, i32)> = db_clone.hscan(&sub).await.unwrap();
                match live.get_state() {
                    LiveState::Running => (),
                    LiveState::Ended | LiveState::TimedOut => {
                        while let Some((chat_id, msg_id)) = iter.next_item().await {
                            let msg = live.to_string();
                            log::info!("Sending message: {msg}");
                            bot.send_message(chat_id.clone(), msg)
                                .disable_link_preview(true).reply_to(MessageId(msg_id)).await.unwrap();
                            redis::pipe().atomic()
                                .hset("subs", &sub, "")
                                .hset(&sub, chat_id, 0)
                                .exec_async(db).await.unwrap();
                        }
                    }
                    LiveState::Unknown(_) => {
                        while let Some((chat_id, msg_id)) = iter.next_item().await {
                            let msg = live.to_string();
                            log::info!("Sending message: {msg}");
                            bot.send_message(chat_id, msg).reply_to(MessageId(msg_id)).await.unwrap();
                        }
                    }
                }
            }
        }
    }
    for live in api.user_live_status(subs).await {
        let sub = live.to_sub();
        let subscribers: Vec<String> = db.hkeys(&sub).await.unwrap();
        for chat_id in subscribers {
            let msg_text = live.to_string();
            log::info!("Sending message: {msg_text}");
            let msg = match platform {
                Platform::TwitterSpace => bot.send_document(chat_id.clone(), live.get_attachment())
                    .caption(msg_text).await.unwrap(),
                Platform::BilibiliLive => bot.send_photo(chat_id.clone(), live.get_attachment())
                    .caption(msg_text).await.unwrap()
            };
            redis::pipe().atomic()
                .hset("subs", &sub, live.get_id())
                .hset(&sub, chat_id, msg.id.0)
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
