use chrono::serde::ts_milliseconds;
use chrono::{DateTime, Utc};
use futures_util::stream::{Stream, StreamExt as _, TryStreamExt as _};
use log::{error, warn};
use reqwest::{Client, Error, IntoUrl, RequestBuilder, Response};
use serde::Deserialize;
use std::collections::HashMap;
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
use std::str::FromStr;

use crate::game_visitor::get_games;
use crate::game_visitor::{GameResult, MoveCounter};
use crate::score::SUS_SCORE;

struct Zuliprc {
    email: String,
    api_key: String,
}

impl Zuliprc {
    fn auth(&self) -> String {
        format!("{}:{}", self.email, self.api_key)
    }
}

pub struct Zulip {
    http: Client,
    token: Zuliprc,
}

// impl Zulip {
//     async fn post<T: IntoUrl + Copy>(&self, url: T, body: String) -> Response {
//         self.req(self.http.post(url).body(body)).await
//     }

//     async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
//         self.req(self.http.get(url)).await
//     }

//     async fn req(&self, builder: RequestBuilder) -> Response {
//         let backoff_factor = 10;
//         let mut sleep_time = Duration::from_secs(60);
//         let max_sleep = Duration::from_secs(3600);
//         loop {
//             match self
//                 .req_inner(builder.try_clone().expect("No streaming body"))
//                 .await
//             {
//                 Ok(resp) => return resp,
//                 Err(err) => error!(
//                     "Error: {}, on request {:?} retrying after: {:?}",
//                     err, &builder, &sleep_time
//                 ),
//             }
//             sleep(sleep_time).await;
//             sleep_time *= backoff_factor;
//             sleep_time = min(sleep_time, max_sleep);
//         }
//     }

//     async fn req_inner(&self, mut builder: RequestBuilder) -> Result<Response, Error> {
//         if let Some(token) = &self.token {
//             builder = builder.bearer_auth(token)
//         }
//         builder.send().await.map(Response::error_for_status)?
//     }

//     pub async fn get_arenas(&self) -> Arenas {
//         self.get("https://lichess.org/api/tournament")
//             .await
//             .json::<Arenas>()
//             .await
//             .expect("Valid JSON Arena decoding")
//     }
// }

// impl Default for Lichess {
//     fn default() -> Self {
//         Self {
//             http: Client::new(),
//             token: fs::read(repo_dir().join("LICHESS_TOKEN.txt"))
//                 .map(|s| String::from_utf8_lossy(&s).to_string())
//                 .ok(),
//         }
//     }
// }
