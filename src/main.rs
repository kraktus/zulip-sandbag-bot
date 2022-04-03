use env_logger::{Builder, Target};
use log::{debug, LevelFilter};

mod game_visitor;
mod lichess;
mod score;
mod setting;
mod util;
mod zulip;

use tokio::time::sleep;

use crate::{lichess::Lichess, setting::Settings};

#[tokio::main]
async fn main() {
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
    lichess.on_start().await;
    loop {
        lichess.watch().await;
        debug!("Waiting {:?} before screening Arenas again.", &s.sleep_time);
        sleep(s.sleep_time).await;
    }
}
