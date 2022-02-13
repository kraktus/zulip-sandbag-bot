use env_logger::{Builder, Target};
use futures_util::StreamExt;
use log::LevelFilter;

mod game_visitor;
mod lichess;
mod score;
mod setting;
mod util;
mod zulip;

use crate::lichess::Lichess;
use crate::setting::Settings;

#[tokio::main]
async fn main() {
    let mut builder = Builder::new();
    let s = Settings::new();
    print!("{s:?}");
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
