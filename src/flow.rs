// SPDX-License-Identifier: GPL-3.0-or-later
//! Portable game flow reconstructed from runtime screens and binary-derived
//! FUN_29120, FUN_20ee0, FUN_20be0, FUN_1fa00/1fa80. No engine source used.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    Normal,
    Challenge,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Title,
    Sets,
    Levels,
    Intro,
    Playing,
    Paused,
    Result,
    SetComplete,
    GameOver,
    Replays,
    Options,
    Help,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Success,
    FallOut,
    TimeOut,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelSpec {
    pub id: String,
    pub bonus: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SetSpec {
    pub id: String,
    pub levels: Vec<LevelSpec>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub player: String,
    pub elapsed_ms: u32,
    pub coins: u32,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Records {
    pub best_times: Vec<Record>,
    pub most_coins: Vec<Record>,
    pub fast_unlock: Vec<Record>,
}
impl Records {
    pub fn submit(&mut self, record: Record, goal_started_open: bool) {
        fn put(rows: &mut Vec<Record>, record: Record, coins_first: bool) {
            rows.push(record);
            rows.sort_by(|a, b| {
                if coins_first {
                    b.coins.cmp(&a.coins).then(a.elapsed_ms.cmp(&b.elapsed_ms))
                } else {
                    a.elapsed_ms.cmp(&b.elapsed_ms).then(b.coins.cmp(&a.coins))
                }
            });
            rows.truncate(3);
        }
        put(&mut self.best_times, record.clone(), false);
        put(&mut self.most_coins, record.clone(), true);
        if !goal_started_open {
            put(&mut self.fast_unlock, record, false);
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SetProgress {
    pub unlocked: BTreeSet<String>,
    pub completed: BTreeSet<String>,
    pub records: BTreeMap<String, Records>,
    pub challenge_records: Records,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Progress {
    pub sets: BTreeMap<String, SetProgress>,
}
impl Progress {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Launch {
    pub set_id: String,
    pub level_id: String,
    pub mode: Mode,
    pub goal_open: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub mode: Mode,
    pub set: usize,
    pub level: usize,
    /// Includes the current ball; reference stores two spare balls initially.
    pub balls: u32,
    pub coins: u32,
    pub elapsed_ms: u32,
    pub goal_started_open: bool,
    start_balls: u32,
    start_coins: u32,
    start_elapsed_ms: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LevelResult {
    pub outcome: Outcome,
    pub coins: u32,
    pub elapsed_ms: u32,
    pub extra_balls: u32,
    pub next_level: Option<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Flow {
    pub sets: Vec<SetSpec>,
    pub progress: Progress,
    pub screen: Screen,
    pub session: Option<Session>,
    pub result: Option<LevelResult>,
    /// Reference level menu: “Goal State in Completed Levels”.
    pub lock_completed_goals: bool,
}
impl Flow {
    pub fn new(sets: Vec<SetSpec>, progress: Progress) -> Self {
        let mut out = Self {
            sets,
            progress,
            screen: Screen::Title,
            session: None,
            result: None,
            lock_completed_goals: true,
        };
        for set in &out.sets {
            if let Some(first) = set.levels.first() {
                out.progress
                    .sets
                    .entry(set.id.clone())
                    .or_default()
                    .unlocked
                    .insert(first.id.clone());
            }
        }
        out
    }
    pub fn unlocked(&self, set: usize, level: usize) -> bool {
        self.sets
            .get(set)
            .and_then(|s| s.levels.get(level).map(|l| (s, l)))
            .map(|(s, l)| {
                self.progress
                    .sets
                    .get(&s.id)
                    .is_some_and(|p| p.unlocked.contains(&l.id))
            })
            .unwrap_or(false)
    }
    pub fn start(&mut self, set: usize, level: usize, mode: Mode) -> Result<Launch, String> {
        let level = if mode == Mode::Challenge { 0 } else { level };
        if !self.unlocked(set, level) {
            return Err("Level is locked or missing".into());
        }
        self.session = Some(Session {
            mode,
            set,
            level,
            balls: 3,
            coins: 0,
            elapsed_ms: 0,
            goal_started_open: false,
            start_balls: 3,
            start_coins: 0,
            start_elapsed_ms: 0,
        });
        self.launch()
    }
    fn launch(&mut self) -> Result<Launch, String> {
        let s = self.session.as_mut().ok_or("No active session")?;
        let set = self.sets.get(s.set).ok_or("Unknown set")?;
        let level = set.levels.get(s.level).ok_or("Unknown level")?;
        let completed = self
            .progress
            .sets
            .get(&set.id)
            .is_some_and(|p| p.completed.contains(&level.id));
        s.goal_started_open = s.mode == Mode::Normal && completed && !self.lock_completed_goals;
        s.start_balls = s.balls;
        s.start_coins = s.coins;
        s.start_elapsed_ms = s.elapsed_ms;
        self.result = None;
        self.screen = Screen::Intro;
        Ok(Launch {
            set_id: set.id.clone(),
            level_id: level.id.clone(),
            mode: s.mode,
            goal_open: s.goal_started_open,
        })
    }
    pub fn begin_play(&mut self) -> bool {
        if self.screen != Screen::Intro {
            return false;
        }
        self.screen = Screen::Playing;
        true
    }
    pub fn pause(&mut self) -> bool {
        if self.screen != Screen::Playing {
            return false;
        }
        self.screen = Screen::Paused;
        true
    }
    pub fn resume(&mut self) -> bool {
        if self.screen != Screen::Paused {
            return false;
        }
        self.screen = Screen::Playing;
        true
    }
    pub fn finish(
        &mut self,
        outcome: Outcome,
        coins: u32,
        elapsed_ms: u32,
        player: &str,
    ) -> Result<&LevelResult, String> {
        if self.screen != Screen::Playing {
            return Err("A result requires active play".into());
        }
        // Reference score tables have centisecond resolution (FUN_20ee0).
        let elapsed_ms = elapsed_ms / 10 * 10;
        let s = self.session.as_mut().ok_or("No active session")?;
        let set = &self.sets[s.set];
        let level = &set.levels[s.level];
        let progress = self.progress.sets.entry(set.id.clone()).or_default();
        let mut next = s.level + 1;
        let mut extra = 0;
        s.elapsed_ms = s.elapsed_ms.saturating_add(elapsed_ms);
        match outcome {
            Outcome::Success => {
                let before = s.coins / 100;
                s.coins = s.coins.saturating_add(coins);
                extra = s.coins / 100 - before;
                s.balls = s.balls.saturating_add(extra);
                progress.completed.insert(level.id.clone());
                progress
                    .records
                    .entry(level.id.clone())
                    .or_default()
                    .submit(
                        Record {
                            player: player.into(),
                            elapsed_ms,
                            coins,
                        },
                        s.goal_started_open,
                    );
                while next < set.levels.len() && set.levels[next].bonus {
                    if s.mode == Mode::Challenge {
                        progress.unlocked.insert(set.levels[next].id.clone());
                    } else if progress.unlocked.contains(&set.levels[next].id) {
                        break;
                    }
                    next += 1;
                }
                if next < set.levels.len() {
                    progress.unlocked.insert(set.levels[next].id.clone());
                } else if s.mode == Mode::Challenge {
                    progress.challenge_records.submit(
                        Record {
                            player: player.into(),
                            elapsed_ms: s.elapsed_ms,
                            coins: s.coins,
                        },
                        false,
                    );
                }
            }
            Outcome::FallOut | Outcome::TimeOut => {
                if s.mode == Mode::Challenge {
                    s.balls = s.balls.saturating_sub(1);
                }
                while next < set.levels.len() && !progress.unlocked.contains(&set.levels[next].id) {
                    next += 1;
                }
            }
        }
        let next_level = (next < set.levels.len()).then_some(next);
        self.result = Some(LevelResult {
            outcome,
            coins,
            elapsed_ms,
            extra_balls: extra,
            next_level,
        });
        self.screen = if s.mode == Mode::Challenge && s.balls == 0 {
            Screen::GameOver
        } else {
            Screen::Result
        };
        Ok(self.result.as_ref().unwrap())
    }
    pub fn retry(&mut self) -> Result<Launch, String> {
        if !matches!(
            self.screen,
            Screen::Result | Screen::Paused | Screen::Playing | Screen::Intro
        ) {
            return Err("Retry is unavailable".into());
        }
        let s = self.session.as_mut().ok_or("No active session")?;
        // Runtime challenge R while active leaves timer/ball unchanged.
        if s.mode == Mode::Challenge && self.screen != Screen::Result {
            return Err("Challenge retry requires a result".into());
        }
        // FUN_20be0 rolls back successful-attempt totals, avoiding reward farming.
        if self
            .result
            .as_ref()
            .is_some_and(|r| r.outcome == Outcome::Success)
        {
            s.balls = s.start_balls;
            s.coins = s.start_coins;
            s.elapsed_ms = s.start_elapsed_ms;
        }
        if s.mode == Mode::Challenge && s.balls == 0 {
            return Err("No balls remain".into());
        }
        self.launch()
    }
    // Advance the game session, rather than iterate a collection.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Result<Option<Launch>, String> {
        if self.screen != Screen::Result {
            return Err("Next level requires a result".into());
        }
        let r = self.result.as_ref().ok_or("No result")?;
        let s = self.session.as_mut().ok_or("No active session")?;
        if s.mode == Mode::Challenge && r.outcome != Outcome::Success {
            return Err("Retry the challenge level".into());
        }
        if let Some(next) = r.next_level {
            s.level = next;
            self.launch().map(Some)
        } else {
            self.screen = Screen::SetComplete;
            Ok(None)
        }
    }
    pub fn leave(&mut self) {
        self.session = None;
        self.result = None;
        self.screen = Screen::Levels;
    }
}

impl Records {
    /// FUN_31620 names initial ranks Hard/Medium/Easy; FUN_1fb00 fills
    /// metadata thresholds and supplies the time limit / coin goal for a
    /// two-value list's Easy entry. Unspecified time defaults to59999cs.
    pub fn reference_level(level: &crate::content::LevelSpec) -> Self {
        fn rows(values: &[i32], fallback: i32, coin_table: bool, other: i32) -> Vec<Record> {
            ["Hard", "Medium", "Easy"]
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let value = values
                        .get(i)
                        .copied()
                        .unwrap_or(if values.len() == 2 && i == 2 {
                            fallback
                        } else {
                            if coin_table {
                                0
                            } else {
                                59999
                            }
                        })
                        .max(0) as u32;
                    Record {
                        player: (*name).into(),
                        elapsed_ms: if coin_table {
                            other.max(0) as u32 * 10
                        } else {
                            value * 10
                        },
                        coins: if coin_table {
                            value
                        } else {
                            other.max(0) as u32
                        },
                    }
                })
                .collect()
        }
        Self {
            best_times: rows(&level.time_records, level.time_centiseconds, false, 0),
            fast_unlock: rows(
                &level.goal_records,
                level.time_centiseconds,
                false,
                level.goal_coins,
            ),
            most_coins: rows(
                &level.coin_records,
                level.goal_coins,
                true,
                if level.time_centiseconds > 0 {
                    level.time_centiseconds
                } else {
                    59999
                },
            ),
        }
    }
    pub fn reference_set(set: &crate::content::LevelSet) -> Self {
        let make = |coin_table: bool| {
            ["Hard", "Medium", "Easy"]
                .iter()
                .enumerate()
                .map(|(i, name)| Record {
                    player: (*name).into(),
                    elapsed_ms: if coin_table {
                        3599990
                    } else {
                        if set.records[i] > 0 {
                            set.records[i] as u32 * 10
                        } else {
                            3599990
                        }
                    },
                    coins: if coin_table {
                        set.records[i + 3].max(0) as u32
                    } else {
                        0
                    },
                })
                .collect()
        };
        Self {
            best_times: make(false),
            most_coins: make(true),
            fast_unlock: vec![],
        }
    }
}
