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

pub fn log_and_pass<T: StdError>(err: T) -> T {
    warn!("{err}");
    err
}

pub fn repo_dir() -> PathBuf {
    env::current_exe()
        .expect("No permission?")
        .ancestors()
        .nth(2)
        .expect("repo dir")
        .to_path_buf()
}

pub async fn req(client: &Client, builder: RequestBuilder, auth: &Option<String>) -> Response {
    let backoff_factor = 10;
    let mut sleep_time = Duration::from_secs(60);
    let max_sleep = Duration::from_secs(3600);
    loop {
        match req_inner(
            client,
            builder.try_clone().expect("No streaming body"),
            auth,
        )
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

async fn req_inner(
    client: &Client,
    mut builder: RequestBuilder,
    auth: &Option<String>,
) -> Result<Response, Error> {
    if let Some(token) = auth {
        builder = builder.bearer_auth(token)
    }
    builder.send().await.map(Response::error_for_status)?
}
