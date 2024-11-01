use futures::StreamExt;
use redis::{aio::MultiplexedConnection, AsyncCommands};
use teloxide::{prelude::*, types::{InlineKeyboardButton, InlineKeyboardMarkup}, utils::command::BotCommands, RequestError};

use crate::subscription::{parse_url, Subscription};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Start the bot
    Start,
    /// Print the help message
    Help,
    #[command(parse_with = parse_url)]
    /// Subscribe to the live stream from the specified URL
    Sub(Subscription),
    #[command(parse_with = parse_url)]
    /// Remove subscription to the live stream from the specified URL
    Del(Subscription),
    /// List existing subscriptions
    List
}

fn make_reply_markup(action: &str) -> InlineKeyboardMarkup {
    let keyboard: [[InlineKeyboardButton; 2]; 1] = [
        [
            InlineKeyboardButton::callback("✅ Confirm", action.to_owned()),
            InlineKeyboardButton::callback("❌ Cancel", "cancel".to_owned())
        ]
    ];
    InlineKeyboardMarkup::new(keyboard)
}

async fn send_reply(
    bot: Bot, chat_id: ChatId, sub: Subscription, mut db: MultiplexedConnection, text: &str, action: &str
) -> Result<Message, RequestError> {
    let reply = bot.send_message(
        chat_id,
        format!("Please confirm that you want to {text} to {} user: {}", sub.platform, sub.user_id)
    ).reply_markup(make_reply_markup(action)).await?;
    let key = format!("{}:{}", reply.chat.id, reply.id);
    redis::pipe().atomic().set(&key, sub.to_string()).expire(&key, 86400).exec_async(&mut db).await.unwrap();
    Ok(reply)
}

pub async fn command_handler(bot: Bot, msg: Message, cmd: Command, mut db: MultiplexedConnection) -> Result<(), RequestError> {
    match cmd {
        Command::Start => bot.send_message(
            msg.chat.id, "Welcome to the Telescope bot. You can view a list of available commands using the /help command."
        ).await?,
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::Sub(sub) => {
            if let Ok(result) = db.sismember(msg.chat.id.to_string(), sub.to_string()).await {
                if result {
                    bot.send_message(msg.chat.id, "You have already subscribed to the user").await?
                } else {
                    send_reply(bot, msg.chat.id, sub, db, "subscribe", "sub").await?
                }
            } else {
                bot.send_message(msg.chat.id, "Database error").await?
            }
        }
        Command::Del(sub) => {
            if let Ok(result) = db.sismember::<_, _, bool>(msg.chat.id.to_string(), sub.to_string()).await {
                if !result {
                    bot.send_message(msg.chat.id, "You are not subscribed to the user").await?
                } else {
                    send_reply(bot, msg.chat.id, sub, db, "unsubscribe", "del").await?
                }
            } else {
                bot.send_message(msg.chat.id, "Database error").await?
            }
        }
        Command::List => {
            if let Ok(results) = db.sscan::<_, String>(msg.chat.id.to_string()).await {
                let subs = results.enumerate().map(|(i, r)| format!("{}. {r}", i + 1)).collect::<Vec<String>>().await.join("\n");
                bot.send_message(msg.chat.id, format!("Your subscriptions:\n{subs}")).await?
            } else {
                bot.send_message(msg.chat.id, "Database error").await?
            }
        }
    };
    respond(())
}
