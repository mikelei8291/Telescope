use log::error;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use teloxide::{
    payloads::AnswerCallbackQuerySetters, prelude::Requester, types::CallbackQuery, utils::markdown::escape,
    RequestError,
};

use crate::{subscription::Subscription, Bot};

pub async fn callback_handler(bot: Bot, query: CallbackQuery, mut db: MultiplexedConnection) -> Result<(), RequestError> {
    if let Some(ref data) = query.data {
        let Some(msg) = query.regular_message() else {
            bot.answer_callback_query(&query.id).text("Message expired, please use the command again").await?;
            return Ok(());
        };
        let key = format!("{}:{}", msg.chat.id, msg.id);
        if data == "cancel" { // handle cancel callback first
            bot.edit_message_text(msg.chat.id, msg.id, "Cancelled").await?;
            let _: () = db.del(key).await.unwrap();
            return Ok(());
        }
        let Ok(sub_str): Result<Option<String>, _> = db.get(&key).await else {
            bot.answer_callback_query(&query.id).text("Database error").await?;
            return Ok(());
        };
        if let Some(sub_str) = sub_str {
            let Ok(sub) = sub_str.parse::<Subscription>() else {
                bot.answer_callback_query(&query.id).text("Database error").await?;
                error!("Wrong, too wrong");
                return Ok(());
            };
            let mut pipe = redis::pipe();
            let pipe = pipe.atomic();
            let (text, pipe) = match data.as_str() {
                "sub" => {
                    let pipe = pipe
                        .hset("subs", &sub_str, "")
                        .hset(&sub_str, &query.from.id.to_string(), 0)
                        .sadd(&query.from.id.to_string(), &sub_str);
                    (format!(
                        "You have successfully subscribed to *{}* user: *{}*",
                        sub.platform,
                        escape(sub.user.username.as_str())
                    ), pipe)
                }
                "del" => {
                    let mut pipe = pipe
                        .srem(&query.from.id.to_string(), &sub_str)
                        .hdel(&sub_str, &query.from.id.to_string());
                    if db.hlen::<_, u64>(&sub_str).await.unwrap() == 1 {
                        pipe = pipe.hdel("subs", &sub_str)
                    }
                    (format!(
                        "You have successfully unsubscribed to *{}* user: *{}*",
                        sub.platform,
                        escape(sub.user.username.as_str())
                    ), pipe)
                }
                _ => ("Why are we still here? Just to suffer?".to_owned(), pipe)
            };
            pipe.del(key).exec_async(&mut db).await.unwrap();
            bot.edit_message_text(msg.chat.id, msg.id, text).await?;
        } else {
            bot.answer_callback_query(&query.id).text("Message expired, please use the command again").await?;
        }
    }
    Ok(())
}
