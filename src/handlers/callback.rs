
use redis::{aio::MultiplexedConnection, AsyncCommands, RedisError};
use teloxide::{payloads::AnswerCallbackQuerySetters, prelude::Requester, types::CallbackQuery, Bot, RequestError};

use crate::subscription::Subscription;

pub async fn callback_handler(bot: Bot, query: CallbackQuery, mut db: MultiplexedConnection) -> Result<(), RequestError> {
    if let Some(ref data) = query.data {
        let Some(msg) = query.regular_message() else {
            bot.answer_callback_query(&query.id).text("Message expired, please use the command again").await?;
            return Ok(());
        };
        if data == "cancel" { // handle cancel callback first
            bot.edit_message_text(msg.chat.id, msg.id, "Cancelled").await?;
            return Ok(());
        }
        let key = format!("{}:{}", msg.chat.id, msg.id);
        let Ok(sub_str): Result<Option<String>, RedisError> = db.get(&key).await else {
            bot.answer_callback_query(&query.id).text("Database error").await?;
            return Ok(());
        };
        if let Some(sub_str) = sub_str {
            let Ok(sub) = sub_str.parse::<Subscription>() else {
                panic!("Wrong, too wrong");
            };
            let mut pipe = redis::pipe().atomic().to_owned();
            let (text, pipe) = match data.as_str() {
                "sub" => {
                    let pipe = pipe
                        .sadd(format!("sub:{sub_str}"), &query.from.id.to_string())
                        .sadd(&query.from.id.to_string(), &sub_str);
                    (format!("You have successfully subscribed to {} user: {}", sub.platform, sub.user_id), pipe)
                }
                "del" => {
                    let pipe = pipe
                        .srem(format!("sub:{sub_str}"), &query.from.id.to_string())
                        .srem(&query.from.id.to_string(), &sub_str);
                    (format!("You have successfully unsubscribed to {} user: {}", sub.platform, sub.user_id), pipe)
                }
                _ => ("Why are we still here? Just to suffer?".to_owned(), &mut pipe)
            };
            pipe.del(key).exec_async(&mut db).await.unwrap();
            bot.edit_message_text(msg.chat.id, msg.id, text).await?;
        } else {
            bot.answer_callback_query(&query.id).text("Message expired, please use the command again").await?;
        }
    }
    Ok(())
}
