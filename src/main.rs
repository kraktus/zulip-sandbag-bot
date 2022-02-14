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

use tokio::time::{sleep, Duration};

// #[tokio::main]
async fn main_() {
    let mut builder = Builder::new();
    let s = Settings::new().expect("syntaxically correct config");
    builder
        .filter(
            None,
            if s.debug {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            },
        )
        .default_format()
        .target(Target::Stdout)
        .init();
    let lichess = Lichess::new(s.clone());
    // let arenas = lichess.get_arenas().await;
    // let mut stream = lichess.get_players(&arenas.finished[0]).await;
    // while let Some(player) = stream.next().await {
    //     println!("{player:?}");
    // }
    lichess.watch().await;
    // let z = Zulip::new(s.zulip);
    // z.post_sandbag_msg("test").await;
}

#[tokio::main]
async fn main() {
    tokio::spawn(async {
        // Process each socket concurrently.
        sleep(Duration::from_millis(10 * 1000)).await;
        println!("2. In spawn, after 10s");
    });
    println!("1. Off spawn");
    sleep(Duration::from_millis(15 * 1000)).await;
    println!("3. Off spawn, after 15s");
}
