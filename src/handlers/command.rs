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

pub async fn command_handler(bot: Bot, msg: Message, cmd: Command) -> Result<(), RequestError> {
    match cmd {
        Command::Start => bot.send_message(
            msg.chat.id, "Welcome to the Telescope bot. You can view a list of available commands using the /help command."
        ).await?,
        Command::Help => bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?,
        Command::Sub(sub) => bot.send_message(
            msg.chat.id,
            format!("Please confirm that you want to subscribe to {} user: {}", sub.platform, sub.user_id)
        ).reply_markup(make_reply_markup("sub")).await?,
        Command::Del(sub) => bot.send_message(
            msg.chat.id,
            format!("Please confirm that you want to unsubscribe to {} user: {}", sub.platform, sub.user_id)
        ).reply_markup(make_reply_markup("del")).await?,
        Command::List => bot.send_message(msg.chat.id, "Your subscriptions:").await?
    };
    respond(())
}
