use std::{future::Future, io::Error, num::NonZero};

use redis::{aio::MultiplexedConnection, AsyncTypedCommands, RedisResult};
use teloxide::{
    payloads::AnswerCallbackQuerySetters,
    prelude::Requester,
    types::{CallbackQuery, Message},
    utils::markdown::escape,
    RequestError
};

use super::command::Action;
use crate::{subscription::fmt_subscriptions, Bot};

const EXPIRED_MESSAGE: &str = "Message expired, please use the command again";

async fn error_callback_query(bot: &Bot, query: &CallbackQuery, msg: &Message, text: &str) -> Result<(), RequestError> {
    bot.answer_callback_query(query.id.clone()).text(text).await?;
    bot.edit_message_text(msg.chat.id, msg.id, text).await?;
    Ok(())
}

async fn try_db<RV>(r: impl Future<Output = RedisResult<RV>>, bot: &Bot, query: &CallbackQuery) -> Result<RV, RequestError> {
    match r.await {
        Ok(o) => Ok(o),
        Err(e) => {
            bot.answer_callback_query(query.id.clone()).text(format!("Database error: {}", escape(&e.to_string()))).await?;
            Err(RequestError::Io(Error::new(std::io::ErrorKind::Other, "Database error").into()))
        }
    }
}

pub async fn callback_handler(bot: Bot, query: CallbackQuery, mut db: MultiplexedConnection) -> Result<(), RequestError> {
    let Some(msg) = query.regular_message() else {
        return bot.answer_callback_query(query.id).text(EXPIRED_MESSAGE).await.and(Ok(()));
    };
    let Some(data) = &query.data else {
        return bot.answer_callback_query(query.id).text("Invalid callback data").await.and(Ok(()));
    };
    let key = format!("{}:{}", msg.chat.id, msg.id);
    if data == "cancel" {  // handle cancel callback first
        try_db(db.del(&key), &bot, &query).await?;
        return error_callback_query(&bot, &query, msg, "Cancelled").await;
    }
    let len = NonZero::new(try_db(db.llen(&key), &bot, &query).await?);
    if len.is_none() {
        return error_callback_query(&bot, &query, msg, EXPIRED_MESSAGE).await;
    }
    let subs = try_db(db.lpop(&key, len), &bot, &query).await?;
    let mut pipe = redis::pipe();
    let mut pipe = pipe.atomic();
    let (text, pipe) = match data.parse() {
        Ok(Action::Subscribe) => {
            for sub in &subs {
                pipe = pipe
                    .hset("subs", sub, "")
                    .hset(sub, query.from.id.to_string(), 0)
                    .sadd(query.from.id.to_string(), sub);
            }
            (format!("You have successfully subscribed to:\n{}", fmt_subscriptions(&subs)), pipe)
        }
        Ok(Action::Unsubscribe) => {
            for sub in &subs {
                pipe = pipe
                    .srem(query.from.id.to_string(), sub)
                    .hdel(sub, query.from.id.to_string());
                if try_db(db.hlen(sub), &bot, &query).await? == 1 {
                    pipe = pipe.hdel("subs", sub)
                }
            }
            (format!("You have successfully unsubscribed to:\n{}", fmt_subscriptions(&subs)), pipe)
        }
        Err(_) => ("Why are we still here? Just to suffer?".to_owned(), pipe)
    };
    try_db(pipe.del(key).exec_async(&mut db), &bot, &query).await?;
    bot.edit_message_text(msg.chat.id, msg.id, text).await?;
    Ok(())
}
