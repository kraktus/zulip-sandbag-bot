

use futures_util::stream::{StreamExt as _};

use reqwest::{Client, IntoUrl, Response};

use std::collections::HashMap;



use tokio::io::AsyncBufReadExt as _;






use std::fs::File;






use crate::util::repo_dir;
use crate::util::req;
use std::io::BufRead;
use std::io::BufReader;

struct Zuliprc {
    email: String,
    api_key: String,
}

impl Default for Zuliprc {
    fn default() -> Self {
        let file = File::open(repo_dir().join("zuliprc.txt")).expect("zuliprc.txt at repo root");
        let reader = BufReader::new(file);
        let mut hash_map: HashMap<String, String> = HashMap::new();

        reader.lines().into_iter().for_each(|l| {
            l.unwrap()
                .split_once("=")
                .map(|(a, b)| hash_map.insert(a.to_string(), b.to_string()));
        });
        Self {
            email: hash_map
                .get("email")
                .expect("email section in zuliprc")
                .clone(),
            api_key: hash_map.get("key").expect("key section in zuliprc").clone(),
        }
    }
}

impl Zuliprc {
    fn auth(&self) -> Option<String> {
        Some(format!("{}:{}", self.email, self.api_key))
    }
}

pub struct Zulip {
    http: Client,
    token: Zuliprc,
}

impl Zulip {
    async fn post<T: IntoUrl + Copy>(&self, url: T, body: String) -> Response {
        req(
            &self.http,
            self.http.post(url).body(body),
            &self.token.auth(),
        )
        .await
    }

    async fn get<T: IntoUrl + Copy>(&self, url: T) -> Response {
        req(&self.http, self.http.get(url), &self.token.auth()).await
    }

    //pub async fn send_report(&self, )
}
