use std::time::Duration;

use redis::aio::MultiplexedConnection;
use tokio::{task, time};

use crate::Bot;

pub mod twitter_space;

pub fn watch(mut db: MultiplexedConnection, bot: Bot) -> task::JoinHandle<()> {
    task::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        loop {
            twitter_space::check(&mut db, &bot).await;
            interval.tick().await;
        }
    })
}
