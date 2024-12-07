use redis::{aio::MultiplexedConnection, AsyncCommands};
use strum::IntoEnumIterator;
use teloxide::{
    payloads::SendMessageSetters,
    prelude::Requester,
    respond,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message},
    utils::{command::BotCommands, markdown::escape},
    RequestError
};

use crate::{platform::Platform, subscription::Subscription, Bot};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Start the bot
    Start,
    /// Print the help message
    Help,
    /// Subscribe to the live stream from the specified URL\. e\.g\. `/sub https://twitter.com/username`
    Sub(String),
    /// Remove subscription to the live stream from the specified URL\. e\.g\. `/del https://twitter.com/username`
    Del(String),
    /// List existing subscriptions
    List,
    /// List all supported platforms
    Platform
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
    bot: &Bot, chat_id: ChatId, sub: Subscription, db: &mut MultiplexedConnection, text: &str, action: &str
) -> Result<Message, RequestError> {
    let reply = bot.send_message(
        chat_id,
        format!("Please confirm that you want to {text} to *{}* user: *{}*", sub.platform, escape(sub.user.username.as_str()))
    ).reply_markup(make_reply_markup(action)).await?;
    let key = format!("{}:{}", reply.chat.id, reply.id);
    redis::pipe().atomic().set(&key, sub.to_db_string()).expire(&key, 86400).exec_async(db).await.unwrap();
    Ok(reply)
}

async fn parse_urls(bot: &Bot, msg: &Message, urls: String) -> Vec<Subscription> {
    let mut subs = vec![];
    let mut errors = vec![];
    for url in urls.split(" ") {
        match Subscription::from_url(url.to_owned()).await {
            Ok(sub) => subs.push(sub),
            Err(e) => errors.push(format!("{url}: {e}")),
        }
    }
    if errors.len() != 0 {
        bot.send_message(msg.chat.id, escape(errors.join("\n").as_str())).await.unwrap();
    }
    subs
}

pub async fn command_handler(bot: Bot, msg: Message, cmd: Command, mut db: MultiplexedConnection) -> Result<(), RequestError> {
    match cmd {
        Command::Start => bot.send_message(
            msg.chat.id, "Welcome to the Telescope bot\\. You can view a list of available commands using the /help command\\."
        ).await?,
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::Sub(urls) => {
            for sub in parse_urls(&bot, &msg, urls).await {
                if let Ok(result) = db.sismember(msg.chat.id.to_string(), sub.to_db_string()).await {
                    if result {
                        bot.send_message(msg.chat.id, format!("You have already subscribed to {sub}")).await?
                    } else {
                        send_reply(&bot, msg.chat.id, sub, &mut db, "subscribe", "sub").await?
                    }
                } else {
                    bot.send_message(msg.chat.id, "Database error").await?
                };
            }
            return respond(());
        }
        Command::Del(urls) => {
            for sub in parse_urls(&bot, &msg, urls).await {
                if let Ok(result) = db.sismember::<_, _, bool>(msg.chat.id.to_string(), sub.to_db_string()).await {
                    if !result {
                        bot.send_message(msg.chat.id, format!("You are not subscribed to {sub}")).await?
                    } else {
                        send_reply(&bot, msg.chat.id, sub, &mut db, "unsubscribe", "del").await?
                    }
                } else {
                    bot.send_message(msg.chat.id, "Database error").await?
                };
            }
            return respond(());
        }
        Command::List => {
            if let Ok(results) = db.smembers::<_, Vec<String>>(msg.chat.id.to_string()).await {
                let subs = results.iter().enumerate().map(|(i, r)| {
                    let Ok(sub) = r.parse::<Subscription>() else {
                        return "".to_string();
                    };
                    format!("{}\\. {sub}", i + 1)
                }).collect::<Vec<String>>().join("\n");
                if subs == "".to_owned() {
                    bot.send_message(msg.chat.id, escape("You have no subscriptions.\nUse the /sub command to add new subscriptions.")).await?
                } else {
                    bot.send_message(msg.chat.id, format!("Your subscriptions:\n{subs}")).await?
                }
            } else {
                bot.send_message(msg.chat.id, "Database error").await?
            }
        }
        Command::Platform => {
            let platforms = Platform::iter().enumerate()
                .map(|(i, p)| format!("{}\\. {p}", i + 1))
                .collect::<Vec<String>>()
                .join("\n");
            bot.send_message(msg.chat.id, format!("Supported platforms:\n{platforms}")).await?
        }
    };
    respond(())
}
