use env_logger::{Builder, Target};
use log::LevelFilter;
use std::collections::HashMap;

mod lichess;
mod score;

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
    let arenas = lichess.get_arenas().await;
    // println!("{:?}", lichess.get_players(&arenas.finished[0].id).await);
}
