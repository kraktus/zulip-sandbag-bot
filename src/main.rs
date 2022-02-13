use env_logger::{Builder, Target};
use futures_util::{Stream, StreamExt};
use log::LevelFilter;
use std::collections::HashMap;
use std::pin::Pin;

mod game_visitor;
mod lichess;
mod score;
mod zulip;
mod util;

use crate::lichess::Lichess;

#[tokio::main]
async fn main() {
    let mut builder = Builder::new();
    builder
        .filter(None, LevelFilter::Trace)
        .default_format()
        .target(Target::Stdout)
        .init();
    let lichess = Lichess::default();
    // let arenas = lichess.get_arenas().await;
    // let mut stream = lichess.get_players(&arenas.finished[0]).await;
    // while let Some(player) = stream.next().await {
    //     println!("{player:?}");
    // }
    println!("{:?}", lichess.get_users_info(&vec!["german11"]).await);
}
