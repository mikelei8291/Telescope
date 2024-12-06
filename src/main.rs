use handlers::{callback::callback_handler, command::{command_handler, Command}};
use log::warn;
use teloxide::{
    adaptors::DefaultParseMode,
    dispatching::UpdateFilterExt,
    filter_command,
    prelude::{Dispatcher, LoggingErrorHandler, Requester, RequesterExt},
    respond,
    types::{Message, ParseMode, Update},
    utils::command::BotCommands,
    RequestError
};
use watchers::watch;

type Bot = DefaultParseMode<teloxide::Bot>;

mod handlers;
mod platform;
mod subscription;
mod apis;
mod watchers;

#[tokio::main]
async fn main() -> Result<(), RequestError> {
    pretty_env_logger::init();
    let bot = teloxide::Bot::from_env().parse_mode(ParseMode::MarkdownV2);
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();
    let db = client.get_multiplexed_async_connection().await.unwrap();
    bot.set_my_commands(Command::bot_commands()).await.expect("Loading bot commands failed.");
    let handler = dptree::entry().branch(
        Update::filter_message().branch(
            filter_command::<Command, _>().endpoint(command_handler)
        ).endpoint(|bot: Bot, msg: Message| async move {
            bot.send_message(msg.chat.id, "Invalid command").await?;
            respond(())
        })
    ).branch(
        Update::filter_callback_query().endpoint(callback_handler)
    );
    let watcher = watch(db.clone(), bot.clone());
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .default_handler(|update| async move { warn!("Unhandled update: {update:?}") })
        .error_handler(LoggingErrorHandler::with_custom_text("Dispatcher error"))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    watcher.await.unwrap();
    Ok(())
}
