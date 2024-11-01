use handlers::{callback::callback_handler, command::{command_handler, Command}};
use log::warn;
use teloxide::{
    dispatching::{HandlerExt, MessageFilterExt, UpdateFilterExt},
    prelude::{Dispatcher, LoggingErrorHandler, Requester},
    types::{Message, Update},
    utils::command::BotCommands,
    Bot, RequestError,
};

mod handlers;
mod subscription;

#[tokio::main]
async fn main() -> Result<(), RequestError> {
    let bot = Bot::from_env();
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let db = client.get_multiplexed_async_connection().await.unwrap();
    bot.set_my_commands(Command::bot_commands()).await.expect("Loading bot commands failed.");
    let handler = dptree::entry().branch(
        Update::filter_message().branch(
            Message::filter_text().filter_command::<Command>().endpoint(command_handler)
        )
    ).branch(
        Update::filter_callback_query().endpoint(callback_handler)
    );
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .default_handler(|update| async move { warn!("Unhandled update: {update:?}") })
        .error_handler(LoggingErrorHandler::with_custom_text("Dispatcher error"))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}
