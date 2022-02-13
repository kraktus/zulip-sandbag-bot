use crate::lichess::log_and_pass;
use pgn_reader::{BufferedReader, RawHeader, SanPlus, Skip, Visitor};
use std::collections::HashMap;
use std::io;

type GameId = String;

#[derive(Debug, Hash, Copy, Clone)]
pub struct GameResult {
    pub moves: usize,
    pub won: bool,
}

#[derive(Debug, Clone)]
struct TempGame {
    pub id: Option<GameId>,
    pub counter: usize,
    pub won: Option<bool>,
    pub is_white: Option<bool>,
}

impl Default for TempGame {
    fn default() -> Self {
        Self {
            id: None,
            counter: 0,
            won: None,
            is_white: None,
        }
    }
}
#[derive(Debug, Hash, Copy, Clone)]
struct TempGameError;

impl TryInto<GameResult> for TempGame {
    type Error = TempGameError;

    fn try_into(self) -> Result<GameResult, Self::Error> {
        Ok(GameResult {
            moves: self.counter,
            won: self.won.ok_or(TempGameError)?,
        })
    }
}

#[derive(Debug)]
pub struct MoveCounter {
    user_id: String,
    moves: HashMap<GameId, GameResult>,
    // internals
    temp: TempGame,
}

impl MoveCounter {
    fn new(user_id: String) -> Self {
        Self {
            user_id,
            moves: HashMap::new(),
            temp: TempGame::default(),
        }
    }
}

impl Visitor for MoveCounter {
    type Result = ();

    fn begin_game(&mut self) {
        self.temp = TempGame::default();
    }

    fn header(&mut self, key: &[u8], value: RawHeader<'_>) {
        let value_opt = value.decode_utf8().map_err(log_and_pass).ok();
        match key {
            b"Site" => self.temp.id = value_opt.and_then(|s| s.split('/').next_back().map(|s| s.to_string())),
            b"White" => self.temp.is_white = value_opt.map(|s| s.contains(&self.user_id)),
            b"Result" => self.temp.won = None, //self.temp.won = value_opt.zip(self.temp.is_white).map(|v, is_white| if is_white {s == "1-0"),
            _ => ()
        }
    }

    fn san(&mut self, _: SanPlus) {
        self.temp.counter += 1;
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        self.temp.clone().id.as_ref().map(|id| {
            self.temp
                .clone()
                .try_into()
                .ok()
                .map(|res| self.moves.insert(id.clone(), res))
        });
    }
}

pub fn nb_sus_games(games: String, user_id: &str) -> MoveCounter {
    let mut reader = BufferedReader::new_cursor(&games[..]);

    let mut counter = MoveCounter::new(user_id.to_string());
    let moves = reader.read_all(&mut counter).expect("valid pgn");
    counter
}
