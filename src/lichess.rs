use log::error;
use reqwest::{Client, Error, IntoUrl, RequestBuilder, Response};
use serde::Deserialize;
use std::env;
use std::fs;
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

use std::cmp::min;

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
#[serde(rename_all = "camelCase")]
pub struct Arena {
    pub id: String,
    pub has_max_rating: Option<bool>, // if not None, should always be true
}

// {"rank":2,"score":57,"rating":2611,"username":"xxx","performance":2462}
#[derive(Deserialize, Debug)]
pub struct Player {
    pub rank: usize,
    pub score: usize,
    pub rating: usize,
    pub username: String,
    pub performance: usize,
}

impl Lichess {
    async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
        let backoff_factor = 10;
        let mut sleep_time = Duration::from_secs(1);
        let max_sleep = Duration::from_secs(3600);
        loop {
            match self.req_inner(url).await {
                Ok(resp) => return resp,
                Err(err) => error!("{err}"),
            }
            sleep(sleep_time).await;
            sleep_time *= backoff_factor;
            sleep_time = min(sleep_time, max_sleep);
        }
    }

    async fn req_inner<T: IntoUrl + Copy>(&self, url: T) -> Result<Response, Error> {
        self.http
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map(Response::error_for_status)?
    }

    pub async fn get_arenas(&self) -> Arenas {
        self.get("https://lichess.org/api/tournament")
            .await
            .json::<Arenas>()
            .await
            .expect("Valid JSON Arena decoding")
    }

    // pub async fn get_players(&self, arena_id: &str) -> Vec<Player> {
    //     // Thanks niklas, https://github.com/lichess-org/lila-openingexplorer/blob/d1b55a43eb4bbaace45c244d7f33d86b11c7ee41/src/indexer/lila.rs#L34-L73
    //     let stream = self.get(&format!("https://lichess.org/api/tournament/{arena_id}/results"))
    //         .await
    //         .bytes_stream()
    //         .expect("Valid bytes_stream");


    // }

    //pub  fn investigate_players(&self, )
}

impl Default for Lichess {
    fn default() -> Self {
        Self {
            http: Client::new(),
            token: fs::read(repo_dir().join("LICHESS_TOKEN.txt")).map(|s| String::from_utf8_lossy(&s).to_string()).ok()
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
