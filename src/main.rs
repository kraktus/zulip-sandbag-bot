use env_logger::{Builder, Target};
use log::{LevelFilter, info};

mod game_visitor;
mod lichess;
mod score;
mod setting;
mod util;
mod zulip;

use crate::lichess::Lichess;
use crate::setting::Settings;

use tokio::time::sleep;

#[tokio::main]
async fn main() {
    // comment test artefact fixed
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
        info!("Waiting {:?} before screening Arenas again.", &s.sleep_time);
        sleep(s.sleep_time).await;
    }
}
