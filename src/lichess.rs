use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use futures_util::stream::{Stream, StreamExt as _, TryStreamExt as _};

use reqwest::{IntoUrl, Response};
use serde::Deserialize;
use std::collections::HashMap;

use tokio::io::AsyncBufReadExt as _;

use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

use tokio::time::timeout;

use std::io;
use std::str::FromStr;
use std::time::Duration;

use crate::game_visitor::get_games;
use crate::game_visitor::MoveCounter;
use crate::score::SUS_SCORE;
use crate::util::{log_and_pass, req, Auth};
use crate::zulip::Zulip;
use crate::Settings;

pub struct Lichess {
    zulip: Zulip,
    token: Option<Auth>,
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
    pub full_name: String,
}

impl Arena {
    pub fn rating_limit(&self) -> Option<usize> {
        if self.has_max_rating {
            usize::from_str(&self.full_name[1..5]).ok()
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
    pub fn new(settings: Settings) -> Self {
        Self {
            zulip: Zulip::new(settings.zulip.clone()),
            token: settings.lichess_token.map(Auth::Bearer),
        }
    }
    async fn post<T: IntoUrl + Copy>(&self, url: T, body: String) -> Response {
        req(
            &self.zulip.http,
            self.zulip.http.post(url).body(body),
            &self.token,
        )
        .await
    }

    async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
        req(&self.zulip.http, self.zulip.http.get(url), &self.token).await
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

    pub async fn get_user_games(&self, user_id: &str, perf: &str) -> Option<MoveCounter> {
        let games = timeout(
            Duration::from_secs(60),
            self.get(
            &format!("https://lichess.org/api/games/user/{user_id}?max=100&rated=true&perfType={perf}&ongoing=false")
        ),
        )
        .await.ok()?
        .text()
        .await.ok()?;
        Some(get_games(games, user_id))
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
                    let mut sus_games = self
                        .get_user_games(&player.username, &arena.perf.key)
                        .await
                        .unwrap_or_else(|| MoveCounter::new(player.username.clone()))
                        .games;
                    sus_games.sort_by(|a, b| a.moves.cmp(&b.moves));
                    print!("{sus_games:?}");
                    let user = self.get_users_info(&[&player.username]).await; // TODO use tokio spawn?
                    if SUS_SCORE
                        .high
                        .perf(&arena.schedule.speed)
                        .map(|score| score <= player.score)
                        .unwrap_or(false)
                    {
                        self.zulip.post_report(&player, &arena, sus_games).await;
                    // send to zulip if arena sort by itself is enough
                    } else if user
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
                        self.zulip.post_report(&player, &arena, sus_games).await;
                    } else if user
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
                        self.zulip.post_report(&player, &arena, sus_games).await;
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
