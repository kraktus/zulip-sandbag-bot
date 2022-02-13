use reqwest::{Client, IntoUrl, Response};

use log::debug;
use serde::Deserialize;

use crate::game_visitor::GameResult;
use crate::lichess::{Arena, Player};
use crate::util::req;

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
    pub http: Client,
    config: ZulipConfig,
}

impl Zulip {
    pub fn new(config: ZulipConfig) -> Self {
        Self {
            config,
            http: Client::new(),
        }
    }

    async fn post<T: IntoUrl + Copy>(&self, url: T, body: String) -> Response {
        req(
            &self.http,
            self.http.post(url).body(body),
            &self.config.auth(),
        )
        .await
    }

    async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
        req(&self.http, self.http.get(url), &self.config.auth()).await
    }

    pub async fn post_report(&self, player: &Player, arena: &Arena, games: Vec<GameResult>) {
        let user_id = &player.username;
        let user_rating = &player.rating;
        let user_score = &player.score;
        let arena_id = &arena.id;
        let arena_fullname = &arena.fullname;
        let perf = &arena.perf.key;
        let body = format!("
**{user_id} ({user_rating})**
{user_id} scored {user_score} in [{arena_fullname}](https://lichess.org/tournament/{arena_id})
*Quick {perf} losses*:
{}...
{}
[short games](<https://lichess.org/@/{user_id}/search?turnsMax=20&perf={perf}&mode=1&players.a={user_id}&players.loser={user_id}&sort.field=t&sort.order=asc)
[all games](<https://lichess.org/mod/{user_id}/games?speed={perf}>).", games.iter().map(
        |g| format!("[{}](<https://lichess.org/{}{}#{}>),", 
            g.moves / 2,
            g.id,
            if !g.is_white {"black"} else {""},
            g.moves)
        ).collect::<String>(),
    "TODO"
    );
        debug!("body sent to zulip: {body}")
    }
    //  f"[{round(SusGame['Moves']/2)}](<https://lichess.org/{SusGame['ID']}{'' if SusGame['UserIsWhite'] else '/black'}#{SusGame['Moves']}>), "
    //  f"...., [short games](<https://lichess.org/@/{UserID.lower()}/search?turnsMax=20&perf={PerfMap[ArenaVariant]}&mode=1&players.a={UserID.lower()}&players.loser={UserID.lower()}&sort.field=t&sort.order=asc>), "
    // f"[all games](<https://lichess.org/mod/{UserID.lower()}/games?speed={ArenaVariant}>)."
}
