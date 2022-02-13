use reqwest::{Client, IntoUrl, Response};

use log::debug;
use serde::Deserialize;

use crate::game_visitor::GameResult;
use crate::lichess::{Arena, Player};
use crate::util::{req, Auth};

#[derive(Debug, Deserialize, Clone)]
pub struct ZulipConfig {
    email: String,
    key: String,
    channel: String,
    topic: String,
    site: String,
}

impl ZulipConfig {
    fn auth(&self) -> Option<Auth> {
        Some(Auth::Basic(self.email.clone(), self.key.clone()))
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

    pub async fn post_sandbag_msg(&self, msg: &str) -> Response {
        // DEBUG set as public
        let params = [
            ("type", "stream"),
            ("to", &self.config.channel),
            ("topic", &self.config.topic),
            ("content", &msg),
        ];
        trace!("Zulip request parameters: {params:?}");
        req(
            &self.http,
            self.http
                .post(format!("{}/api/v1/messages", self.config.site))
                .form(&params),
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
        let arena_fullname = &arena.full_name;
        let perf = &arena.perf.key;
        let msg = format!("
**[{user_id} ({user_rating})](https://lichess.org/@/{user_id})**
{user_id} scored {user_score} in [{arena_fullname}](https://lichess.org/tournament/{arena_id})
*Quick {perf} losses*:
{}...
[short games](<https://lichess.org/@/{user_id}/search?turnsMax=20&perf={perf}&mode=1&players.a={user_id}&players.loser={user_id}&sort.field=t&sort.order=asc)
[all games](<https://lichess.org/mod/{user_id}/games?speed={perf}>)", games.iter().take(6).map(
        |g| format!("[{}](<https://lichess.org/{}{}#{}>),", 
            g.moves / 2,
            g.id,
            if !g.is_white {"black"} else {""},
            g.moves)
        ).collect::<String>()
    );
        debug!("body sent to zulip: {msg}");
        self.post_sandbag_msg(&msg).await;
    }
    //  f"[{round(SusGame['Moves']/2)}](<https://lichess.org/{SusGame['ID']}{'' if SusGame['UserIsWhite'] else '/black'}#{SusGame['Moves']}>), "
    //  f"...., [short games](<https://lichess.org/@/{UserID.lower()}/search?turnsMax=20&perf={PerfMap[ArenaVariant]}&mode=1&players.a={UserID.lower()}&players.loser={UserID.lower()}&sort.field=t&sort.order=asc>), "
    // f"[all games](<https://lichess.org/mod/{UserID.lower()}/games?speed={ArenaVariant}>)."
}
