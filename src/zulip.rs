use log::{debug, info, trace};
use reqwest::{Client, Response};
use serde::Deserialize;

use crate::{
    game_visitor::GameResult,
    lichess::{Arena, Player},
    util::{perf_to_index, req, Auth},
};

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

    async fn post_sandbag_msg(&self, msg: &str) -> Response {
        let params = [
            ("type", "stream"),
            ("to", &self.config.channel),
            ("topic", &self.config.topic),
            ("content", msg),
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

    pub async fn start_message(&self) {
        let start_message = format!("(re)starting! commit {}", env!("GIT_HASH"));
        info!("{}", &start_message);
        self.post_sandbag_msg(&start_message).await;
    }

    pub async fn post_report(&self, player: &Player, arena: &Arena, games: Vec<GameResult>) {
        let user_id = &player.username;
        let user_rating = &player.rating;
        let user_score = &player.score;
        let arena_id = &arena.id;
        let arena_fullname = &arena.full_name;
        let perf = &arena.perf.key;
        let perf_index = perf_to_index(perf)
            .map(|x| x.to_string())
            .unwrap_or_else(|| "?".to_string());
        let msg = format!("
**[{user_id} ({user_rating})](https://lichess.org/@/{user_id})**
{user_id} scored {user_score} in [{arena_fullname}](https://lichess.org/tournament/{arena_id})
*Quick {perf} losses*:
{}...
[short games](https://lichess.org/@/{user_id}/search?turnsMax=20&perf={perf_index}&mode=1&players.a={user_id}&players.loser={user_id}&sort.field=t&sort.order=asc)
[all games](https://lichess.org/mod/{user_id}/games?speed={perf})", games.iter().take(6).map(
        |g| format!("[{}](<https://lichess.org/{}{}#{}>),", 
            g.moves / 2,
            g.id,
            if !g.is_white {"/black"} else {""},
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
