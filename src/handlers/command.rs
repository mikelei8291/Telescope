use redis::aio::MultiplexedConnection;
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

pub async fn command_handler(bot: Bot, msg: Message, cmd: Command, db: MultiplexedConnection) -> Result<(), RequestError> {
    match cmd {
        Command::Start => bot.send_message(
            msg.chat.id, "Welcome to the Telescope bot. You can view a list of available commands using the /help command."
        ).await?,
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::Sub(sub) => send_reply(bot, msg.chat.id, sub, db, "subscribe", "sub").await?,
        Command::Del(sub) => send_reply(bot, msg.chat.id, sub, db, "unsubscribe", "del").await?,
        Command::List => bot.send_message(msg.chat.id, "Your subscriptions:").await?
    };
    respond(())
}
