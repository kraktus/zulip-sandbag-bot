use futures_util::stream::StreamExt as _;

use reqwest::{Client, IntoUrl, Response};

use std::collections::HashMap;

use tokio::io::AsyncBufReadExt as _;

use std::fs::File;
use serde::Deserialize;

use crate::util::repo_dir;
use crate::util::req;
use std::io::BufRead;
use std::io::BufReader;

#[derive(Debug, Deserialize, Clone)]
pub struct ZulipConfig {
    email: String,
    key: String,
}

impl ZulipConfig {
    fn auth(&self) -> Option<String> {
        Some(format!("{}:{}", self.email, self.key))
    }
}

pub struct Zulip {
    http: Client,
    token: ZulipConfig,
}

impl Zulip {
    async fn post<T: IntoUrl + Copy>(&self, url: T, body: String) -> Response {
        req(
            &self.http,
            self.http.post(url).body(body),
            &self.token.auth(),
        )
        .await
    }

    async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
        req(&self.http, self.http.get(url), &self.token.auth()).await
    }

    async fn post_report(&self) {todo!()}
    // GamesContent = f"*Quick {ArenaVariant} losses*: "
    //  f"[{round(SusGame['Moves']/2)}](<https://lichess.org/{SusGame['ID']}{'' if SusGame['UserIsWhite'] else '/black'}#{SusGame['Moves']}>), "
    //  f"...., [short games](<https://lichess.org/@/{UserID.lower()}/search?turnsMax=20&perf={PerfMap[ArenaVariant]}&mode=1&players.a={UserID.lower()}&players.loser={UserID.lower()}&sort.field=t&sort.order=asc>), "
    // f"[all games](<https://lichess.org/mod/{UserID.lower()}/games?speed={ArenaVariant}>)."
    // pub async fn send_report(&self)
}
