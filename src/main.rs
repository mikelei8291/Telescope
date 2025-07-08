use std::{env, panic, process::exit};

use handlers::{callback::callback_handler, command::{command_handler, Command}};
use log::warn;
use teloxide::{
    adaptors::DefaultParseMode,
    dispatching::UpdateFilterExt,
    filter_command,
    prelude::{Dispatcher, LoggingErrorHandler, Request, Requester, RequesterExt},
    types::{Message, ParseMode, Update},
    utils::{command::BotCommands, markdown::{code_block, escape}},
    RequestError
};
use tokio::{runtime::Handle, task::block_in_place};
use watcher::watch;

type Bot = DefaultParseMode<teloxide::Bot>;

mod handlers;
mod platform;
mod subscription;
mod apis;
mod watcher;
mod log_utils;

#[tokio::main]
async fn main() -> Result<(), RequestError> {
    pretty_env_logger::init();
    let bot = teloxide::Bot::from_env().parse_mode(ParseMode::MarkdownV2);
    const REDIS_ERROR_MSG: &str = "Failed to connect to redis server";
    let client = redis::Client::open("redis://127.0.0.1/").expect(REDIS_ERROR_MSG);
    let db = client.get_multiplexed_async_connection().await.expect(REDIS_ERROR_MSG);
    let panic_bot = bot.clone();
    let hook = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        hook(info);
        block_in_place(|| Handle::current().block_on(
            panic_bot.send_message(env::var("BOT_OWNER").ok()?, code_block(format!("{info}").as_str())).send()
        ).ok());
        exit(1);
    }));
    bot.set_my_commands(Command::bot_commands()).await.expect("Loading bot commands failed.");
    let handler = dptree::entry().branch(
        Update::filter_message().branch(
            filter_command::<Command, _>().endpoint(command_handler)
        ).endpoint(async |bot: Bot, msg: Message|
            bot.send_message(msg.chat.id, escape("Sorry, I don't understand.")).await.and(Ok(()))
        )
    ).branch(
        Update::filter_callback_query().endpoint(callback_handler)
    );
    watch(db.clone(), bot.clone());
    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![db])
        .default_handler(async |update| warn!("Unhandled update: {update:?}"))
        .error_handler(LoggingErrorHandler::with_custom_text("Dispatcher error"))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
    Ok(())
}
