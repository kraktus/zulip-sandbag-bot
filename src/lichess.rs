use std::{collections::HashMap, io, str::FromStr, time::Duration};

use chrono::{serde::ts_milliseconds, DateTime, Utc};
use futures_util::stream::{Stream, StreamExt as _, TryStreamExt as _};
use log::{debug, info, warn};
use reqwest::{Error, IntoUrl, Response};
use serde::Deserialize;
use tokio::{io::AsyncBufReadExt as _, time::timeout};
use tokio_stream::wrappers::LinesStream;
use tokio_util::io::StreamReader;

use crate::{
    game_visitor::{get_games, MoveCounter},
    score::SusScore,
    util::{log_and_pass, req, Auth},
    zulip::Zulip,
    Settings,
};

pub struct Lichess {
    zulip: Zulip,
    token: Option<Auth>,
    sus_score: SusScore,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct Arenas {
    pub created: Vec<Arena>,
    pub finished: Vec<Arena>,
}

#[derive(Deserialize, Debug, Default)]
pub struct Perf {
    pub key: String, // TODO use enum instead
}

#[derive(Deserialize, Debug, Default)]
pub struct Schedule {
    pub freq: String,
    pub speed: String,
}

// schedule":{"freq":"hourly","speed":"hyperBullet"}
#[derive(Deserialize, Debug, Default)]
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
    pub fn rating_limit(&self) -> Option<u16> {
        if self.has_max_rating {
            // safer version of &self.full_name[1..5]
            u16::from_str(&self.full_name.chars().skip(1).take(4).collect::<String>()).ok()
        } else {
            None
        }
    }
}

// {"rank":2,"score":57,"rating":2611,"username":"xxx","performance":2462}
#[derive(Deserialize, Debug)]
pub struct Player {
    pub rank: u16,
    pub score: u16,
    pub rating: u16,
    pub username: String,
    pub performance: Option<u16>,
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
        info!("Score threshold used for reporting: {:?}", settings.score);
        Self {
            zulip: Zulip::new(settings.zulip.clone()),
            token: settings.lichess_token.map(Auth::Bearer),
            sus_score: settings.score,
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

    pub async fn get_users_info(&self, user_ids: &[&str]) -> Result<HashMap<String, User>, Error> {
        self.post(
            "https://lichess.org/api/users",
            user_ids.iter().copied().take(300).collect::<String>(),
        )
        .await
        .json::<Vec<User>>()
        .await
        .map_err(|err| {
            warn!("{err}, requested user ids {user_ids:?}");
            err
        })
        .map(|users| HashMap::from_iter(users.into_iter().map(|u| ((&u.id).to_string(), u))))
    }

    pub async fn get_user_games(&self, user_id: &str, perf: &str) -> Option<MoveCounter> {
        let last_6_months = (Utc::today() - chrono::Duration::days(180)).format("%Y-%m-%d");
        let games = timeout(
            Duration::from_secs(60),
            self.get(
            &format!("https://lichess.org/api/games/user/{user_id}?max=100&rated=true&perfType={perf}&ongoing=false&dateMin={last_6_months}")
        ),
        )
        .await.ok()?
        .text()
        .await.ok()?;
        Some(get_games(games, user_id))
    }

    pub async fn on_start(&self) {
        self.zulip.start_message().await
    }

    #[allow(clippy::blocks_in_if_conditions)] // conflicts with `rustfmt` https://github.com/rust-lang/rust-clippy/issues/8099
    pub async fn watch(&self) {
        debug!("Start screening recent arenas");
        for arena in self
            .get_arenas()
            .await
            .finished
            .iter()
            .filter(|a| a.has_max_rating)
        {
            let mut stream = self.get_players(arena).await;
            while let Some(player) = stream.next().await {
                if self.preselect_player(arena, &player) {
                    let sus_games = self
                        .get_user_games(&player.username, &arena.perf.key)
                        .await
                        .unwrap_or_else(|| MoveCounter::new(player.username.clone()))
                        .get_sorted_sus_games();
                    if let Ok(user) = self.get_users_info(&[&player.username]).await {
                        // TODO use tokio spawn?
                        if self
                            .sus_score
                            .high
                            .perf(&arena.schedule.speed)
                            .map(|score| score <= player.score)
                            .unwrap_or(false)
                        {
                            self.zulip.post_report(&player, arena, sus_games).await;
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
                            self.zulip.post_report(&player, arena, sus_games).await;
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
                            self.zulip.post_report(&player, arena, sus_games).await;
                        }
                    }
                }
            }
        }
        debug!("Finished screening recent arenas")
    }

    fn preselect_player(&self, arena: &Arena, player: &Player) -> bool {
        self.sus_score
            .low
            .perf(&arena.schedule.speed)
            .map(|score| score <= player.score)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::setting::Settings;
    use env_logger::{Builder, Target};
    use log::{debug, LevelFilter};

    #[test]
    fn test_arena_rating_limit() {
        let mut a = Arena::default();
        a.has_max_rating = true;
        a.full_name = "≤1500 Blitz Arena".to_string();
        assert_eq!(a.rating_limit(), Some(1500));
    }

    fn setup_lichess() -> Lichess {
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
        Lichess::new(s)
    }

    #[tokio::test]
    async fn test_get_user_games() {
        let l = setup_lichess();
        l.get_user_games("german11", "bullet").await;
    }

    // #[tokio::test]
    // async fn test_get_user_info_closed_account() {
    //     let l = setup_lichess();
    //     l.get_users_info(&["Closed_Account"]).await;
    // }
}
