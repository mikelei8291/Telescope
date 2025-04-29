use redis::{aio::MultiplexedConnection, AsyncCommands};
use strum::IntoEnumIterator;
use strum_macros::{Display, EnumString};
use teloxide::{
    payloads::SendMessageSetters,
    prelude::Requester,
    sugar::request::RequestLinkPreviewExt,
    types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup, Message},
    utils::{command::BotCommands, markdown::escape},
    RequestError
};

use crate::{
    platform::Platform,
    subscription::{fmt_subscriptions, Subscription},
    Bot
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum Command {
    /// Start the bot
    Start,
    /// Print the help message
    Help,
    /// Subscribe to the live stream from the specified URL. You can specify multiple URLs by separating them by spaces.
    /// e.g. /sub https://twitter.com/username
    Sub(String),
    /// Remove subscription to the live stream from the specified URL. You can specify multiple URLs by separating them by spaces.
    /// e.g. /del https://twitter.com/username
    Del(String),
    /// List existing subscriptions
    List,
    /// List all supported platforms
    Platform
}

#[derive(Display, EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum Action {
    Subscribe,
    Unsubscribe
}

fn make_reply_markup(action: Action) -> InlineKeyboardMarkup {
    let keyboard: [[InlineKeyboardButton; 2]; 1] = [
        [
            InlineKeyboardButton::callback("✅ Confirm", action.to_string()),
            InlineKeyboardButton::callback("❌ Cancel", "cancel".to_owned())
        ]
    ];
    InlineKeyboardMarkup::new(keyboard)
}

async fn send_reply(
    bot: &Bot, chat_id: ChatId, db: &mut MultiplexedConnection, subs: &Vec<Subscription>, action: Action
) -> Result<(), RequestError> {
    let reply = bot.send_message(
        chat_id,
        format!("Please confirm that you want to {action} to the following users:\n{}", fmt_subscriptions(&subs))
    ).reply_markup(make_reply_markup(action)).await?;
    let key = format!("{}:{}", reply.chat.id, reply.id);
    if let Err(e) = redis::pipe().atomic().rpush(&key, subs).expire(&key, 86400).exec_async(db).await {
        bot.edit_message_text(reply.chat.id, reply.id, format!("Database error: {}", escape(&e.to_string()))).await?;
    }
    Ok(())
}

async fn process_urls(
    bot: &Bot, msg: &Message, db: &mut MultiplexedConnection, urls: String, action: Action
) -> Result<(), RequestError> {
    let mut subs = vec![];
    let mut errors = vec![];
    for url in urls.split(" ").filter_map(|url| {
        let url = url.trim();
        (!url.is_empty()).then_some(url)
    }) {
        match Subscription::from_url(url.to_owned()).await {
            Ok(sub) => {
                match (db.sismember(msg.chat.id.0, &sub).await, &action) {
                    (Ok(true), Action::Subscribe) => errors.push(format!("{}: You have already subscribed to {sub}", escape(url))),
                    (Ok(true), Action::Unsubscribe) => subs.push(sub),
                    (Ok(false), Action::Subscribe) => subs.push(sub),
                    (Ok(false), Action::Unsubscribe) => errors.push(format!("{}: You are not subscribed to {sub}", escape(url))),
                    (Err(e), _) => {
                        errors.push(format!("Database error: {}", escape(&e.to_string())));
                        break;
                    }
                }
            },
            Err(e) => errors.push(format!("{}: {}", escape(url), escape(&e.to_string())))
        }
    }
    if errors.len() > 0 {
        bot.send_message(msg.chat.id, errors.join("\n")).disable_link_preview(true).await?;
    }
    if subs.len() > 0 {
        send_reply(bot, msg.chat.id, db, &subs, action).await?;
    }
    if errors.len() == 0 && subs.len() == 0 {
        bot.send_message(msg.chat.id, "Nothing to do").await?;
    }
    Ok(())
}

pub async fn command_handler(bot: Bot, msg: Message, cmd: Command, mut db: MultiplexedConnection) -> Result<(), RequestError> {
    match cmd {
        Command::Start => bot.send_message(
            msg.chat.id, "Welcome to the Telescope bot\\. You can view a list of available commands using the /help command\\."
        ).await?,
        Command::Help => bot.send_message(msg.chat.id, escape(Command::descriptions().to_string().as_str())).await?,
        Command::Sub(urls) => return process_urls(&bot, &msg, &mut db, urls, Action::Subscribe).await,
        Command::Del(urls) => return process_urls(&bot, &msg, &mut db, urls, Action::Unsubscribe).await,
        Command::List => {
            match db.smembers(msg.chat.id.0).await {
                Ok(subs) => {
                    let sub_list = fmt_subscriptions(&subs);
                    if sub_list.is_empty() {
                        bot.send_message(msg.chat.id, escape("You have no subscriptions.\nUse the /sub command to add new subscriptions.")).await?
                    } else {
                        bot.send_message(msg.chat.id, format!("Your subscriptions:\n{sub_list}")).await?
                    }
                },
                Err(e) => bot.send_message(msg.chat.id, format!("Database error: {}", escape(&e.to_string()))).await?
            }
        }
        Command::Platform => {
            let platforms = Platform::iter().enumerate()
                .map(|(i, p)| format!("{}\\. {p}", i + 1)).collect::<Vec<_>>().join("\n");
            bot.send_message(msg.chat.id, format!("Supported platforms:\n{platforms}")).await?
        }
    };
    Ok(())
}
