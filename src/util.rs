use futures_util::stream::StreamExt as _;
use log::{error, warn};
use reqwest::{Client, Error, RequestBuilder, Response};

use tokio::time::{sleep, Duration};

use std::cmp::min;
use std::error::Error as StdError;

pub fn log_and_pass<T: StdError>(err: T) -> T {
    warn!("{err}");
    err
}

pub enum Auth {
    Basic(String, String),
    Bearer(String),
}

pub async fn req(client: &Client, builder: RequestBuilder, auth_opt: &Option<Auth>) -> Response {
    let backoff_factor = 10;
    let mut sleep_time = Duration::from_secs(60);
    let max_sleep = Duration::from_secs(3600);
    loop {
        match req_inner(
            client,
            builder.try_clone().expect("No streaming body"),
            auth_opt,
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
    _client: &Client,
    mut builder: RequestBuilder,
    auth_opt: &Option<Auth>,
) -> Result<Response, Error> {
    if let Some(auth) = auth_opt {
        match auth {
            Auth::Bearer(token) => builder = builder.bearer_auth(token),
            Auth::Basic(username, pwd) => builder = builder.basic_auth(&username, Some(&pwd)),
        }
    }
    builder.send().await.map(Response::error_for_status)?
}

pub fn perf_to_index(s: &str) -> Option<u8> {
    match s {
        "bullet" => Some(1),
        "blitz" => Some(2),
        "classical" => Some(3),
        "rapid" => Some(6),
        _ => None,
    }
}
