use teloxide::{prelude::Requester, types::CallbackQuery, Bot, RequestError};

pub async fn callback_handler(bot: Bot, query: CallbackQuery) -> Result<(), RequestError> {
    if let Some(ref data) = query.data {
        bot.answer_callback_query(&query.id).await?;
        let text = match data.as_str() {
            "sub" => {
                // Do subscribe
                "You have successfully subscribed to {} user: {}"
            }
            "del" => {
                // Do unsubscribe
                "You have successfully unsubscribed to {} user: {}"
            }
            "cancel" => "Cancelled",
            _ => "Why are we still here? Just to suffer?"
        };
        if let Some(msg) = query.regular_message() {
            bot.edit_message_text(msg.chat.id, msg.id, text).await?;
        }
    }
    Ok(())
}
