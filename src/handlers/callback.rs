use std::num::NonZero;

use redis::{aio::MultiplexedConnection, AsyncCommands};
use teloxide::{
    payloads::AnswerCallbackQuerySetters, prelude::Requester, types::{CallbackQuery, Message}, RequestError,
};

use crate::{subscription::fmt_subscriptions, Bot};

use super::command::Action;

const EXPIRED_MESSAGE: &str = "Message expired, please use the command again";

async fn error_callback_query(bot: &Bot, query: &CallbackQuery, msg: Option<&Message>) -> Result<(), RequestError> {
    bot.answer_callback_query(&query.id).text(EXPIRED_MESSAGE).await?;
    if let Some(msg) = msg {
        bot.edit_message_text(msg.chat.id, msg.id, EXPIRED_MESSAGE).await?;
    }
    Ok(())
}

pub async fn callback_handler(bot: Bot, query: CallbackQuery, mut db: MultiplexedConnection) -> Result<(), RequestError> {
    if let Some(ref data) = query.data {
        let Some(msg) = query.regular_message() else {
            return error_callback_query(&bot, &query, None).await;
        };
        let key = format!("{}:{}", msg.chat.id, msg.id);
        if data == "cancel" { // handle cancel callback first
            bot.edit_message_text(msg.chat.id, msg.id, "Cancelled").await?;
            let _: () = db.del(key).await.unwrap();
            return Ok(());
        }
        let len = db.llen(&key).await.ok().map(|l| NonZero::new(l).unwrap());
        let Ok(subs) = db.lpop(&key, len).await else {
            return error_callback_query(&bot, &query, Some(msg)).await;
        };
        let mut pipe = redis::pipe();
        let mut pipe = pipe.atomic();
        let (text, pipe) = match data.parse() {
            Ok(Action::Subscribe) => {
                for sub in &subs {
                    pipe = pipe
                        .hset("subs", sub, "")
                        .hset(sub, &query.from.id.to_string(), 0)
                        .sadd(&query.from.id.to_string(), sub);
                }
                (format!("You have successfully subscribed to:\n{}", fmt_subscriptions(&subs)), pipe)
            }
            Ok(Action::Unsubscribe) => {
                for sub in &subs {
                    pipe = pipe
                        .srem(&query.from.id.to_string(), sub)
                        .hdel(sub, &query.from.id.to_string());
                    if db.hlen::<_, u64>(sub).await.unwrap() == 1 {
                        pipe = pipe.hdel("subs", sub)
                    }
                }
                (format!("You have successfully unsubscribed to:\n{}", fmt_subscriptions(&subs)), pipe)
            }
            Err(_) => ("Why are we still here? Just to suffer?".to_owned(), pipe)
        };
        pipe.del(key).exec_async(&mut db).await.unwrap();
        bot.edit_message_text(msg.chat.id, msg.id, text).await?;
    }
    Ok(())
}
