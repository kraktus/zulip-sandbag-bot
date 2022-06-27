use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct Score {
    pub bullet: u16,
    pub super_blitz: u16,
    pub blitz: u16,
    pub rapid: u16,
}

impl Score {
    pub fn perf(&self, perf: &str) -> Option<u16> {
        match perf {
            "bullet" => Some(self.bullet),
            "superBlitz" => Some(self.super_blitz),
            "blitz" => Some(self.blitz),
            "rapid" => Some(self.rapid),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub struct SusScore {
    pub low: Score,
    pub medium: Score,
    pub high: Score,
}
