use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use futures_util::stream::{Stream, StreamExt as _, TryStreamExt as _};

use reqwest::{Client, IntoUrl, Response};
use serde::Deserialize;
use std::collections::HashMap;

use tokio::io::AsyncBufReadExt as _;

use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

use std::io;
use std::str::FromStr;

use crate::game_visitor::get_games;
use crate::game_visitor::{GameResult, MoveCounter};
use crate::score::SUS_SCORE;
use crate::util::{log_and_pass, req};

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
    fullname: String,
}

impl Arena {
    pub fn rating_limit(&self) -> Option<usize> {
        if self.has_max_rating {
            usize::from_str(&self.fullname[1..5]).ok()
        } else {
            None
        }
    }
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
    pub id: String,
    #[serde(default)]
    pub tos_violation: bool,
    #[serde(with = "ts_milliseconds")]
    pub created_at: DateTime<Utc>,
}

impl User {
    pub fn is_new(&self) -> bool {
        self.created_at > (Utc::now() - chrono::Duration::days(20))
    }

    pub fn is_very_new(&self) -> bool {
        self.created_at > (Utc::now() - chrono::Duration::days(10))
    }
}

impl Lichess {
    pub fn new(token: Option<String>) -> Self {
        Self {
            http: Client::new(),
            token,
        }
    }
    async fn post<T: IntoUrl + Copy>(&self, url: T, body: String) -> Response {
        req(&self.http, self.http.post(url).body(body), &self.token).await
    }

    async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
        req(&self.http, self.http.get(url), &self.token).await
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

    pub async fn get_users_info(&self, user_ids: &[&str]) -> HashMap<String, User> {
        HashMap::from_iter(
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
            .into_iter()
            .map(|u| ((&u.id).to_string(), u)),
        )
    }

    pub async fn get_user_games(&self, user_id: &str, perf: &str) -> MoveCounter {
        get_games(self.get(
            &format!("https://lichess.org/api/games/user/{user_id}?max=100&rated=true&perfType={perf}&ongoing=false")
        )
        .await
        .text()
        .await
        .expect("raw PGN"), user_id)
    }

    async fn send_player(_arena: &Arena, _player: &Player) {
        todo!()
    }

    pub async fn watch(&self) {
        for arena in self
            .get_arenas()
            .await
            .finished
            .iter()
            .filter(|a| a.has_max_rating)
        {
            let mut stream = self.get_players(&arena).await;
            while let Some(player) = stream.next().await {
                if preselect_player(&arena, &player) {
                    let sus_games: Vec<(String, GameResult)> = self
                        .get_user_games(&player.username, &arena.perf.key)
                        .await
                        .games
                        .into_iter()
                        .filter(|(_id, game_res)| game_res.moves < 30 && !game_res.won)
                        .collect();
                    print!("{sus_games:?}");
                    if SUS_SCORE
                        .high
                        .perf(&arena.schedule.speed)
                        .map(|score| score <= player.score)
                        .unwrap_or(false)
                    {
                        todo!() // send to zulip if arena sort by itself is enough
                    }
                    let user = self.get_users_info(&[&player.username]).await; // TODO use tokio spawn?
                    if user
                        .get(&player.username)
                        .map(User::is_new)
                        .unwrap_or(false)
                        || sus_games.len() > 25
                        || arena
                            .rating_limit()
                            .zip(player.performance)
                            .map(|(r, performance)| {
                                player.rating < r - 200 || performance > r + 500
                            })
                            .unwrap_or(false)
                    {
                        todo!()
                    }
                    if user
                        .get(&player.username)
                        .map(User::is_very_new) // different than above
                        .unwrap_or(false)
                        || sus_games.len() > 30
                        || arena
                            .rating_limit()
                            .zip(player.performance)
                            .map(|(r, performance)| {
                                player.rating < r - 300 || performance > r + 400
                            })
                            .unwrap_or(false)
                    {
                        todo!()
                    }
                }
            }
        }
    }
}

fn preselect_player(arena: &Arena, player: &Player) -> bool {
    SUS_SCORE
        .low
        .perf(&arena.schedule.speed)
        .map(|score| score <= player.score)
        .unwrap_or(false)
}
