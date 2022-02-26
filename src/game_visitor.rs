use pgn_reader::{BufferedReader, RawHeader, SanPlus, Skip, Visitor};

use crate::util::log_and_pass;

type GameId = String;

#[derive(Debug, Hash, Clone)]
pub struct GameResult {
    pub id: String,
    pub moves: usize,
    pub won: bool,
    pub is_white: bool,
}

#[derive(Debug, Clone, Default)]
struct TempGame {
    pub id: Option<GameId>,
    pub counter: usize,
    pub won: Option<bool>,
    pub is_white: Option<bool>,
}

#[derive(Debug, Hash, Copy, Clone)]
struct TempGameError;

impl TryInto<GameResult> for TempGame {
    type Error = TempGameError;

    fn try_into(self) -> Result<GameResult, Self::Error> {
        Ok(GameResult {
            id: self.id.ok_or(TempGameError)?,
            moves: self.counter,
            won: self.won.ok_or(TempGameError)?,
            is_white: self.is_white.ok_or(TempGameError)?,
        })
    }
}

#[derive(Debug)]
pub struct MoveCounter {
    pub user_id: String,
    pub games: Vec<GameResult>,
    temp: TempGame,
}

impl MoveCounter {
    pub fn new(user_id: String) -> Self {
        Self {
            user_id,
            games: vec![],
            temp: TempGame::default(),
        }
    }

    pub fn get_sorted_sus_games(&self) -> Vec<GameResult> {
        let mut sus_games: Vec<GameResult> =
            self.games.clone().into_iter().filter(|g| !g.won).collect();
        sus_games.sort_by(|a, b| a.moves.cmp(&b.moves));
        sus_games
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
            b"Site" => {
                self.temp.id =
                    value_opt.and_then(|s| s.split('/').next_back().map(|s| s.to_string()))
            }
            b"White" => self.temp.is_white = value_opt.map(|s| s.contains(&self.user_id)),
            b"Result" => {
                self.temp.won = value_opt.zip(self.temp.is_white).map(|(v, is_white)| {
                    if is_white {
                        v == "1-0"
                    } else {
                        v == "0-1"
                    }
                })
            }
            _ => (),
        }
    }

    fn san(&mut self, _: SanPlus) {
        self.temp.counter += 1;
    }

    fn begin_variation(&mut self) -> Skip {
        Skip(true) // stay in the mainline
    }

    fn end_game(&mut self) -> Self::Result {
        if let Ok(res) = self.temp.clone().try_into() {
            self.games.push(res)
        }
    }
}

pub fn get_games(games: String, user_id: &str) -> MoveCounter {
    let mut reader = BufferedReader::new_cursor(&games[..]);

    let mut counter = MoveCounter::new(user_id.to_string());
    let _moves = reader.read_all(&mut counter).expect("valid pgn");
    counter
}
