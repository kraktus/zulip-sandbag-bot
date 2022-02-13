use futures_util::stream::{Stream, StreamExt as _, TryStreamExt as _};
use log::{error, warn};
use reqwest::{Client, Error, IntoUrl, RequestBuilder, Response};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio::io::AsyncBufReadExt as _;
use tokio::time::{sleep, Duration};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

use std::cmp::min;
use std::error::Error as StdError;
use std::io;

use crate::game_visitor::nb_sus_games;
use crate::game_visitor::MoveCounter;
use crate::score::SUS_SCORE;

pub struct Lichess {
    http: Client,
    token: Option<String>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Arenas {
    pub created: Vec<Arena>,
    pub finished: Vec<Arena>,
}

#[derive(Deserialize, Debug)]
pub struct Perf {
    pub key: String, // TODO use enum instead
}

#[derive(Deserialize, Debug)]
pub struct Schedule {
    pub freq: String,
    pub speed: String,
}

// schedule":{"freq":"hourly","speed":"hyperBullet"}
#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Arena {
    pub id: String,
    #[serde(default)]
    pub has_max_rating: bool, // if not None, should always be true
    pub schedule: Schedule,
    pub perf: Perf,
}

// {"rank":2,"score":57,"rating":2611,"username":"xxx","performance":2462}
#[derive(Deserialize, Debug)]
pub struct Player {
    pub rank: usize,
    pub score: usize,
    pub rating: usize,
    pub username: String,
    pub performance: Option<usize>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub tos_violation: bool,
    pub created_at: Duration,
}

impl Lichess {
    async fn post<T: IntoUrl + Copy>(&self, url: T, body: String) -> Response {
        self.req(self.http.post(url).body(body)).await
    }

    async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
        self.req(self.http.get(url)).await
    }

    async fn req(&self, builder: RequestBuilder) -> Response {
        let backoff_factor = 10;
        let mut sleep_time = Duration::from_secs(60);
        let max_sleep = Duration::from_secs(3600);
        loop {
            match self
                .req_inner(builder.try_clone().expect("No streaming body"))
                .await
            {
                Ok(resp) => return resp,
                Err(err) => error!(
                    "Error: {}, on request {:?} retrying after: {:?}",
                    err, &builder, &sleep_time
                ),
            }
            sleep(sleep_time).await;
            sleep_time *= backoff_factor;
            sleep_time = min(sleep_time, max_sleep);
        }
    }

    async fn req_inner(&self, mut builder: RequestBuilder) -> Result<Response, Error> {
        if let Some(token) = &self.token {
            builder = builder.bearer_auth(token)
        }
        builder.send().await.map(Response::error_for_status)?
    }

    pub async fn get_arenas(&self) -> Arenas {
        self.get("https://lichess.org/api/tournament")
            .await
            .json::<Arenas>()
            .await
            .expect("Valid JSON Arena decoding")
    }

    pub async fn get_players(&self, arena: &Arena) -> impl Stream<Item = Player> {
        // Thanks niklas, https://github.com/lichess-org/lila-openingexplorer/blob/d1b55a43eb4bbaace45c244d7f33d86b11c7ee41/src/indexer/lila.rs#L34-L73
        let stream = self
            .get(&format!(
                "https://lichess.org/api/tournament/{}/results",
                &arena.id
            ))
            .await
            .bytes_stream()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err));

        Box::pin(
            LinesStream::new(StreamReader::new(stream).lines()).filter_map(|line| async move {
                match line {
                    Ok(line) if line.is_empty() => None,
                    Ok(line) => serde_json::from_str::<Player>(&line)
                        .map_err(log_and_pass)
                        .ok(),
                    Err(err) => panic!("{err:?}"),
                }
            }),
        )
    }

    pub async fn get_users_info(&self, user_ids: &[&str]) -> Vec<User> {
        self.post(
            "https://lichess.org/api/users",
            user_ids
                .into_iter()
                .map(|s| *s)
                .take(300)
                .collect::<String>(),
        )
        .await
        .json::<Vec<User>>()
        .await
        .expect("Valid JSON User decoding")
    }

    pub async fn get_user_games(&self, user_id: &str, perf: &str) -> MoveCounter {
        nb_sus_games(self.get(
            &format!("https://lichess.org/api/games/user/{user_id}?max=100&rated=true&perfType={perf}&ongoing=false")
        )
        .await
        .text()
        .await
        .expect("raw PGN"), user_id)
    }

    async fn send_player(arena: &Arena, player: &Player) {
        ()
    }

    pub async fn watch(&self) {
        for arena in self.get_arenas().await.finished {
            let mut stream = self.get_players(&arena).await;
            while let Some(player) = stream.next().await {
                if preselect_player(&arena, &player) {
                    // check if above high score threshold before dling games
                    let counter = self.get_user_games(&player.username, &arena.perf.key);
                }
            }
        }
    }
}

impl Default for Lichess {
    fn default() -> Self {
        Self {
            http: Client::new(),
            token: fs::read(repo_dir().join("LICHESS_TOKEN.txt"))
                .map(|s| String::from_utf8_lossy(&s).to_string())
                .ok(),
        }
    }
}

fn repo_dir() -> PathBuf {
    env::current_exe()
        .expect("No permission?")
        .ancestors()
        .nth(2)
        .expect("repo dir")
        .to_path_buf()
}

fn preselect_player(arena: &Arena, player: &Player) -> bool {
    SUS_SCORE
        .low
        .perf(&arena.schedule.speed)
        .map(|score| score <= player.score)
        .unwrap_or(false)
}

pub fn log_and_pass<T: StdError>(err: T) -> T {
    warn!("{err}");
    err
}
