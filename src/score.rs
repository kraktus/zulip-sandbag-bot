// HighScore   = {"bullet": 45, "superBlitz": 45, "blitz": 40, "rapid": 35}
//    MediumScore = {"bullet": 35, "superBlitz": 35, "blitz": 30, "rapid": 25}
//    LowScore    = {"bullet": 30, "superBlitz": 30, "blitz": 25, "rapid": 20}

pub struct Score {
    pub bullet: usize,
    pub superBlitz: usize,
    pub blitz: usize,
    pub rapid: usize,
}

impl Score {
    pub fn perf(&self, perf: &str) -> usize {
        match perf {
            "bullet" => self.bullet,
            "superBlitz" => self.superBlitz,
            "blitz" => self.blitz,
            "rapid" => self.rapid,
            _ => panic!("perf not found"),
        }
    }
}

pub struct SusScore {
    pub low: Score,
    pub medium: Score,
    pub high: Score,
}

pub const SUS_SCORE: SusScore = SusScore {
    high: Score {
        bullet: 45,
        superBlitz: 45,
        blitz: 40,
        rapid: 35,
    },
    medium: Score {
        bullet: 35,
        superBlitz: 35,
        blitz: 30,
        rapid: 25,
    },
    low: Score {
        bullet: 30,
        superBlitz: 30,
        blitz: 25,
        rapid: 20,
    },
};
