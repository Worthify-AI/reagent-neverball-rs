// SPDX-License-Identifier: GPL-3.0-or-later
//! Menu presentation rebuilt from the running reference and packaged level metadata.
use macroquad::prelude::*;
use reagent_neverball_rs::{
    content::{Catalog, LevelSet, LevelSpec},
    flow::{self, Flow, Mode, Screen},
    settings::Settings,
    sol::Sol,
};
use std::collections::BTreeMap;
pub enum ReplayEndAction {
    None,
    Repeat,
    Delete,
    Keep,
}
pub enum Action {
    None,
    Launch(flow::Launch),
    Resume,
    Retry,
    Next,
    Exit,
    PlayReplay(String),
    SaveReplay(String),
    DeleteReplay(String),
}
#[derive(Clone, Copy, PartialEq)]
enum OptionsPage {
    Main,
    Graphics,
    Configure,
    Player,
    Ball,
    Resolution,
    Bindings,
    Language,
}
pub struct Ui {
    pub catalog: Catalog,
    pub levels: Vec<Vec<LevelSpec>>,
    pub selected_set: usize,
    pub selected_level: usize,
    pub status: String,
    pub replay_name: String,
    options_page: OptionsPage,
    player_edit: String,
    binding_capture: Option<String>,
    binding_since: f64,
    binding_page: usize,
    caps: bool,
    score_mode: usize,
    replay_page: usize,
    translations: BTreeMap<String, BTreeMap<String, String>>,
    active_language: String,
    previews: BTreeMap<String, Texture2D>,
    focus: usize,
    button_index: usize,
}
impl Ui {
    pub async fn load(root: &str) -> Result<Self, String> {
        let text = load_string(&format!("{root}/sets.txt"))
            .await
            .map_err(|e| e.to_string())?;
        let mut sets = Vec::new();
        let mut levels = Vec::new();
        for file in text
            .lines()
            .map(str::trim)
            .filter(|s| !s.is_empty() && !s.starts_with('#'))
        {
            let text = load_string(&format!("{root}/{file}"))
                .await
                .map_err(|e| e.to_string())?;
            let set = LevelSet::parse(file, &text)?;
            let mut specs = Vec::new();
            for path in &set.levels {
                let bytes = load_file(&format!("{root}/{path}"))
                    .await
                    .map_err(|e| e.to_string())?;
                let sol = Sol::from_bytes(&bytes).map_err(|e| e.to_string())?;
                specs.push(LevelSpec::from_sol(path, &sol));
            }
            sets.push(set);
            levels.push(specs);
        }
        let mut translations = BTreeMap::new();
        for (code, _) in LANGUAGES.iter().filter(|(code, _)| *code != "en") {
            if let Ok(text) = load_string(&format!("{root}/locales/{code}.json")).await {
                if let Ok(strings) = serde_json::from_str::<BTreeMap<String, String>>(&text) {
                    translations.insert((*code).to_string(), strings);
                }
            }
        }
        Ok(Self {
            catalog: Catalog { sets },
            levels,
            selected_set: 0,
            selected_level: 0,
            status: String::new(),
            replay_name: "My Replay".into(),
            options_page: OptionsPage::Main,
            player_edit: String::new(),
            binding_capture: None,
            binding_since: 0.,
            binding_page: 0,
            caps: true,
            score_mode: 0,
            replay_page: 0,
            translations,
            active_language: String::new(),
            previews: BTreeMap::new(),
            focus: 0,
            button_index: 0,
        })
    }
    pub fn specs(&self) -> Vec<flow::SetSpec> {
        self.catalog
            .sets
            .iter()
            .zip(&self.levels)
            .map(|(s, ls)| flow::SetSpec {
                id: s.id.clone(),
                levels: ls
                    .iter()
                    .map(|l| flow::LevelSpec {
                        id: l.path.clone(),
                        bonus: l.bonus,
                    })
                    .collect(),
            })
            .collect()
    }
    pub async fn prepare_preview(&mut self, root: &str, screen: Screen) {
        let path = match screen {
            Screen::Sets => self
                .catalog
                .sets
                .get(self.selected_set)
                .map(|s| s.preview.clone()),
            Screen::Levels => self
                .levels
                .get(self.selected_set)
                .and_then(|s| s.get(self.selected_level))
                .map(|s| s.screenshot.clone()),
            _ => None,
        };
        if let Some(path) = path {
            if !path.is_empty() && !self.previews.contains_key(&path) {
                if let Ok(t) = load_texture(&format!("{root}/{path}")).await {
                    self.previews.insert(path, t);
                }
            }
        }
    }
    pub fn reset_focus(&mut self) {
        self.focus = 0;
    }
    fn label(font: &Font, text: &str, x: f32, y: f32, size: u16, color: Color) {
        Self::label_raw(font, &translate(text), x, y, size, color);
    }
    fn label_raw(font: &Font, text: &str, x: f32, y: f32, size: u16, color: Color) {
        crate::paint_text(font, text, x, y, size as f32, color);
    }
    fn centered(font: &Font, text: &str, y: f32, size: u16, color: Color) {
        let text = translate(text);
        let text = text.as_str();
        let w = crate::measure_text_guarded(text, Some(font), size, 1.).width;
        Self::label(font, text, 400. - w / 2., y, size, color);
    }
    fn panel(r: Rect, color: Color) {
        let radius = 10f32.min(r.h / 2.).min(r.w / 2.);
        let center = vec2(r.x + r.w / 2., r.y + r.h / 2.);
        let mut edge = Vec::with_capacity(28);
        for (x, y, start) in [
            (r.x + r.w - radius, r.y + radius, -90f32),
            (r.x + r.w - radius, r.y + r.h - radius, 0.),
            (r.x + radius, r.y + r.h - radius, 90.),
            (r.x + radius, r.y + radius, 180.),
        ] {
            for step in 0..=6 {
                let a = (start + step as f32 * 15.).to_radians();
                edge.push(vec2(x + a.cos() * radius, y + a.sin() * radius));
            }
        }
        for i in 0..edge.len() {
            draw_triangle(center, edge[i], edge[(i + 1) % edge.len()], color);
        }
    }
    fn button(&mut self, font: &Font, r: Rect, text: &str, size: u16, enabled: bool) -> bool {
        self.button_raw(font, r, &translate(text), size, enabled)
    }
    fn button_raw(&mut self, font: &Font, r: Rect, text: &str, size: u16, enabled: bool) -> bool {
        let width = crate::measure_text_guarded(text, Some(font), size, 1.).width;
        let size = if width > r.w - 12. {
            (size as f32 * (r.w - 12.) / width).max(12.) as u16
        } else {
            size
        };
        let (mx, my) = mouse_position();
        let p = vec2(mx / screen_width() * 800., my / screen_height() * 600.);
        let hovered = r.contains(p);
        let index = self.button_index;
        self.button_index += 1;
        if hovered && mouse_delta_position().length_squared() > 0. {
            self.focus = index;
        }
        let active = enabled && (hovered || self.focus == index);
        Self::panel(
            r,
            if active {
                Color::new(0.65, 0.45, 0.19, 0.72)
            } else {
                Color::new(0.12, 0.12, 0.12, 0.72)
            },
        );
        let m = crate::measure_text_guarded(text, Some(font), size, 1.);
        Self::label_raw(
            font,
            text,
            r.x + (r.w - m.width) / 2. + 2.,
            r.y + (r.h + size as f32 * 0.7) / 2. + 2.,
            size,
            Color::new(0., 0., 0., 0.6),
        );
        Self::label_raw(
            font,
            text,
            r.x + (r.w - m.width) / 2.,
            r.y + (r.h + size as f32 * 0.7) / 2.,
            size,
            if enabled { WHITE } else { ORANGE },
        );
        enabled
            && ((hovered && is_mouse_button_pressed(MouseButton::Left))
                || (self.focus == index && is_key_pressed(KeyCode::Enter)))
    }
    fn preview(&self, path: &str, r: Rect) {
        if let Some(t) = self.previews.get(path) {
            draw_texture_ex(
                t,
                r.x,
                r.y,
                WHITE,
                DrawTextureParams {
                    dest_size: Some(vec2(r.w, r.h)),
                    ..Default::default()
                },
            );
        }
    }
    pub fn draw(
        &mut self,
        font: &Font,
        flow: &mut Flow,
        settings: &mut Settings,
        replays: &reagent_neverball_rs::replay::ReplayStore,
    ) -> Action {
        if self.active_language != settings.language {
            TRANSLATIONS.with(|t| {
                *t.borrow_mut() = self
                    .translations
                    .get(&settings.language)
                    .cloned()
                    .unwrap_or_default()
            });
            self.active_language = settings.language.clone();
        }
        set_camera(&Camera2D {
            target: vec2(400., 300.),
            zoom: vec2(2. / 800., 2. / 600.),
            ..Default::default()
        });
        if is_key_pressed(KeyCode::Down) {
            self.focus = self.focus.saturating_add(1);
        }
        if is_key_pressed(KeyCode::Up) {
            self.focus = self.focus.saturating_sub(1);
        }
        self.button_index = 0;
        let mut action = Action::None;
        match flow.screen {
            Screen::Title => {
                Self::panel(
                    Rect::new(100., 75., 600., 115.),
                    Color::new(0., 0., 0., 0.4),
                );
                Self::centered(font, "Neverball", 162., 82, ORANGE);
                for (i, (name, target)) in [
                    ("Play", Screen::Sets),
                    ("Replay", Screen::Replays),
                    ("Help", Screen::Help),
                    ("Options", Screen::Options),
                ]
                .iter()
                .enumerate()
                {
                    if self.button(
                        font,
                        Rect::new(290., 205. + i as f32 * 65., 220., 64.),
                        name,
                        46,
                        true,
                    ) {
                        flow.screen = *target;
                        self.reset_focus();
                    }
                }
                if self.button(font, Rect::new(290., 465., 220., 64.), "Exit", 46, true) {
                    action = Action::Exit;
                }
            }
            Screen::Sets => {
                clear_background(BLACK);
                Self::label(font, "Level Set", 40., 55., 28, ORANGE);
                if self.button(font, Rect::new(670., 20., 90., 40.), "Back", 24, true)
                    || is_key_pressed(KeyCode::Escape)
                {
                    flow.screen = Screen::Title;
                    self.reset_focus();
                }
                for i in 0..self.catalog.sets.len() {
                    if self.catalog.sets[i].id == "misc" {
                        continue;
                    }
                    let title = self.catalog.sets[i].title.clone();
                    if self.button(
                        font,
                        Rect::new(40., 85. + i as f32 * 55., 355., 49.),
                        &title,
                        24,
                        true,
                    ) {
                        self.selected_set = i;
                        self.selected_level = 0;
                        flow.screen = Screen::Levels;
                        self.reset_focus();
                    }
                }
                if let Some(s) = self.catalog.sets.get(self.selected_set) {
                    self.preview(&s.preview, Rect::new(420., 95., 340., 255.));
                    let description = translate(&s.description);
                    let mut lines = Vec::new();
                    for paragraph in description.lines() {
                        let mut line = String::new();
                        for word in paragraph.split_whitespace() {
                            let candidate = if line.is_empty() {
                                word.to_string()
                            } else {
                                format!("{line} {word}")
                            };
                            if !line.is_empty()
                                && crate::measure_text_guarded(&candidate, Some(font), 18, 1.).width
                                    > 340.
                            {
                                lines.push(std::mem::take(&mut line));
                            }
                            if !line.is_empty() {
                                line.push(' ');
                            }
                            line.push_str(word);
                        }
                        lines.push(line);
                    }
                    for (i, line) in lines.iter().take(9).enumerate() {
                        Self::label_raw(font, line, 420., 390. + i as f32 * 22., 18, WHITE);
                    }
                }
            }
            Screen::Levels => {
                clear_background(BLACK);
                let si = self.selected_set;
                let title = self.catalog.sets[si].title.clone();
                Self::label(font, &title, 520., 49., 24, ORANGE);
                if self.button(font, Rect::new(40., 23., 80., 40.), "Back", 24, true)
                    || is_key_pressed(KeyCode::Escape)
                {
                    flow.screen = Screen::Sets;
                    self.reset_focus();
                }
                let count = self.levels[si].len();
                let mut normal = 0;
                let mut bonus = 0;
                for i in 0..count {
                    let b = self.levels[si][i].bonus;
                    let (col, row, label) = if b {
                        let n = bonus;
                        bonus += 1;
                        (
                            4,
                            n,
                            ["I", "II", "III", "IV", "V"]
                                .get(n)
                                .unwrap_or(&"+")
                                .to_string(),
                        )
                    } else {
                        let n = normal;
                        normal += 1;
                        (n % 4, n / 4, format!("{:02}", n + 1))
                    };
                    let enabled = flow.unlocked(si, i);
                    let r = Rect::new(40. + col as f32 * 72., 83. + row as f32 * 46., 71., 45.);
                    let (mx, my) = mouse_position();
                    if r.contains(vec2(
                        mx / screen_width() * 800.,
                        my / screen_height() * 600.,
                    )) {
                        self.selected_level = i;
                    }
                    if self.button(font, r, &label, 24, enabled) {
                        match flow.start(si, i, Mode::Normal) {
                            Ok(l) => action = Action::Launch(l),
                            Err(e) => self.status = e,
                        }
                    }
                }
                if self.button(font, Rect::new(40., 318., 360., 46.), "Challenge", 24, true) {
                    match flow.start(si, 0, Mode::Challenge) {
                        Ok(l) => action = Action::Launch(l),
                        Err(e) => self.status = e,
                    }
                }
                let level = &self.levels[si][self.selected_level];
                self.preview(&level.screenshot, Rect::new(410., 93., 340., 255.));
                Self::panel(
                    Rect::new(125., 375., 300., 148.),
                    Color::new(0.04, 0.04, 0.04, 1.),
                );
                let records = flow
                    .progress
                    .sets
                    .get(&self.catalog.sets[si].id)
                    .and_then(|p| p.records.get(&level.path))
                    .cloned();
                if binding_pressed(settings, "key_score_next") {
                    self.score_mode = (self.score_mode + 1) % 3;
                }
                for (i, name) in ["Most Coins", "Best Times", "Fast Unlock"]
                    .iter()
                    .enumerate()
                {
                    if self.button(
                        font,
                        Rect::new(440., 380. + i as f32 * 45., 210., 42.),
                        name,
                        24,
                        true,
                    ) {
                        self.score_mode = i;
                    }
                }
                if let Some(rows) = records {
                    let rows = match self.score_mode {
                        0 => &rows.most_coins,
                        1 => &rows.best_times,
                        _ => &rows.fast_unlock,
                    };
                    for (i, r) in rows.iter().enumerate() {
                        Self::label(
                            font,
                            &format!("{}  {}  {}", format_time(r.elapsed_ms), r.player, r.coins),
                            130.,
                            420. + i as f32 * 35.,
                            21,
                            WHITE,
                        );
                    }
                }
                Self::label(
                    font,
                    "Goal State in Completed Levels",
                    55.,
                    570.,
                    24,
                    ORANGE,
                );
                if self.button(font, Rect::new(490., 540., 135., 42.), "Locked", 24, true) {
                    flow.lock_completed_goals = true;
                    settings.lock_goals = true;
                }
                if self.button(font, Rect::new(630., 540., 135., 42.), "Unlocked", 24, true) {
                    flow.lock_completed_goals = false;
                    settings.lock_goals = false;
                }
            }
            Screen::Paused => {
                Self::centered(font, "Paused", 185., 64, ORANGE);
                if self.button(font, Rect::new(280., 240., 240., 55.), "Continue", 36, true) {
                    action = Action::Resume;
                }
                if self.button(
                    font,
                    Rect::new(280., 300., 240., 55.),
                    "Restart",
                    36,
                    flow.session
                        .as_ref()
                        .is_some_and(|s| s.mode == Mode::Normal),
                ) {
                    action = Action::Retry;
                }
                if self.button(font, Rect::new(280., 360., 240., 55.), "Exit", 36, true) {
                    flow.leave();
                }
            }
            Screen::Result | Screen::GameOver | Screen::SetComplete => {
                let success = flow
                    .result
                    .as_ref()
                    .is_some_and(|r| r.outcome == flow::Outcome::Success);
                let title = match flow.screen {
                    Screen::GameOver => "Game Over",
                    Screen::SetComplete => "Set Complete",
                    _ => match flow.result.as_ref().map(|r| r.outcome) {
                        Some(flow::Outcome::Success) => "Success!",
                        Some(flow::Outcome::FallOut) => "Fall-out!",
                        _ => "Time's Up!",
                    },
                };
                Self::panel(
                    Rect::new(125., 93., 565., if success { 270. } else { 125. }),
                    Color::new(0., 0., 0., 0.58),
                );
                Self::centered(font, title, 150., 60, if success { GREEN } else { RED });
                if let Some(r) = &flow.result {
                    Self::centered(
                        font,
                        &format!("Time {}   Coins {}", format_time(r.elapsed_ms), r.coins),
                        205.,
                        27,
                        WHITE,
                    );
                }
                if success {
                    if binding_pressed(settings, "key_score_next") {
                        self.score_mode = (self.score_mode + 1) % 3;
                    }
                    let records = flow
                        .session
                        .as_ref()
                        .and_then(|session| {
                            flow.progress
                                .sets
                                .get(&flow.sets[session.set].id)
                                .and_then(|p| {
                                    if flow.screen == Screen::SetComplete {
                                        Some(&p.challenge_records)
                                    } else {
                                        p.records
                                            .get(&flow.sets[session.set].levels[session.level].id)
                                    }
                                })
                        })
                        .cloned();
                    if let Some(records) = records {
                        let rows = match self.score_mode {
                            0 => &records.most_coins,
                            1 => &records.best_times,
                            _ => &records.fast_unlock,
                        };
                        for (i, r) in rows.iter().enumerate() {
                            Self::label(
                                font,
                                &format!(
                                    "{}  {}  {}",
                                    format_time(r.elapsed_ms),
                                    r.player,
                                    r.coins
                                ),
                                150.,
                                260. + i as f32 * 40.,
                                24,
                                ORANGE,
                            );
                        }
                    }
                    for (i, name) in ["Most Coins", "Best Times", "Fast Unlock"]
                        .iter()
                        .enumerate()
                    {
                        if self.button(
                            font,
                            Rect::new(490., 230. + i as f32 * 42., 185., 40.),
                            name,
                            24,
                            true,
                        ) {
                            self.score_mode = i;
                        }
                    }
                }
                if let Some(session) = &flow.session {
                    if session.mode == Mode::Challenge {
                        Self::centered(
                            font,
                            &format!(
                                "Balls {}   Total Coins {}",
                                session.balls.saturating_sub(1),
                                session.coins
                            ),
                            390.,
                            24,
                            WHITE,
                        );
                    }
                }
                if self.button(
                    font,
                    Rect::new(85., 435., 205., 52.),
                    "Save Replay",
                    26,
                    replays.last.is_some(),
                ) {
                    flow.screen = Screen::Replays;
                    self.reset_focus();
                }
                if self.button(
                    font,
                    Rect::new(295., 435., 205., 52.),
                    "Retry Level",
                    26,
                    flow.screen == Screen::Result,
                ) {
                    action = Action::Retry;
                }
                if self.button(
                    font,
                    Rect::new(505., 435., 205., 52.),
                    if success { "Next Level" } else { "Finish" },
                    26,
                    true,
                ) {
                    if success && flow.screen == Screen::Result {
                        action = Action::Next;
                    } else {
                        flow.leave();
                    }
                }
                if self.button(font, Rect::new(300., 505., 200., 45.), "Finish", 26, true)
                    || is_key_pressed(KeyCode::Escape)
                {
                    flow.leave();
                }
            }
            Screen::Help => {
                clear_background(BLACK);
                Self::centered(font, "How to Play", 65., 48, ORANGE);
                for (i, line) in [
                    "Move the mouse or arrow keys to tilt the floor.",
                    "Collect coins to unlock the goal.",
                    "Guide the ball to the goal before time runs out.",
                    "1 Chase     2 Lazy     3 Manual",
                    "S / D rotate the camera; hold Shift to rotate faster.",
                    "Mouse left / right rotate; middle toggles camera.",
                    "E toggles Chase and Manual. Escape pauses.",
                ]
                .iter()
                .enumerate()
                {
                    Self::centered(font, line, 140. + i as f32 * 48., 23, WHITE);
                }
                if self.button(font, Rect::new(330., 500., 140., 50.), "Back", 30, true)
                    || is_key_pressed(KeyCode::Escape)
                {
                    flow.screen = Screen::Title;
                }
            }
            Screen::Options => self.draw_options(font, flow, settings),
            Screen::Replays => {
                clear_background(BLACK);
                Self::centered(font, "Replay", 65., 48, ORANGE);
                if replays.last.is_some()
                    && self.button(font, Rect::new(60., 100., 300., 45.), "Last", 28, true)
                {
                    action = Action::PlayReplay("Last".into());
                }
                let names: Vec<_> = replays.saved.keys().cloned().collect();
                for (i, name) in names.iter().skip(self.replay_page * 5).take(5).enumerate() {
                    if self.button_raw(
                        font,
                        Rect::new(60., 155. + i as f32 * 46., 480., 43.),
                        name,
                        24,
                        true,
                    ) {
                        action = Action::PlayReplay(name.clone());
                    }
                    if self.button(
                        font,
                        Rect::new(555., 155. + i as f32 * 46., 140., 43.),
                        "Delete",
                        24,
                        true,
                    ) {
                        action = Action::DeleteReplay(name.clone());
                    }
                }
                if self.button(
                    font,
                    Rect::new(60., 395., 110., 40.),
                    "Previous",
                    20,
                    self.replay_page > 0,
                ) {
                    self.replay_page = self.replay_page.saturating_sub(1);
                }
                if self.button(
                    font,
                    Rect::new(585., 395., 110., 40.),
                    "Next",
                    20,
                    (self.replay_page + 1) * 5 < names.len(),
                ) {
                    self.replay_page += 1;
                }
                for c in typed_characters() {
                    if !c.is_control() && self.replay_name.len() < 80 {
                        self.replay_name.push(c);
                    }
                }
                if is_key_pressed(KeyCode::Backspace) {
                    self.replay_name.pop();
                }
                Self::panel(
                    Rect::new(60., 450., 480., 42.),
                    Color::new(0.2, 0.2, 0.2, 1.),
                );
                Self::label_raw(font, &self.replay_name, 70., 479., 24, WHITE);
                if self.button(
                    font,
                    Rect::new(555., 450., 180., 42.),
                    "Keep Last",
                    24,
                    replays.last.is_some(),
                ) {
                    action = Action::SaveReplay(self.replay_name.clone());
                }
                Self::centered(font, &self.status, 535., 18, ORANGE);
                if self.button(font, Rect::new(330., 548., 140., 42.), "Back", 26, true)
                    || is_key_pressed(KeyCode::Escape)
                {
                    flow.screen = Screen::Title;
                }
            }
            Screen::Intro | Screen::Playing => {}
        }
        if flow.screen == Screen::Options && !self.status.is_empty() {
            Self::centered(font, &self.status, 575., 18, ORANGE);
        }
        if self.button_index > 0 {
            self.focus = self.focus.min(self.button_index - 1);
        }
        set_default_camera();
        action
    }
}

fn format_time(ms: u32) -> String {
    format!(
        "{}:{:02}.{:02}",
        ms / 60000,
        (ms / 1000) % 60,
        (ms / 10) % 100
    )
}
const BALLS: &[&str] = &[
    "atom",
    "basic-ball",
    "blinky",
    "catseye",
    "cheese-ball",
    "diagonal-ball",
    "earth",
    "eyeball",
    "lava",
    "magic-eightball",
    "melon",
    "orange",
    "reactor",
    "rift",
    "saturn",
    "snowglobe",
    "sootsprite",
    "ufo",
];
impl Ui {
    pub fn ball_preview_requested(&self) -> bool {
        self.options_page == OptionsPage::Ball
    }
    fn option_row(&mut self, font: &Font, y: f32, label: &str, value: &str, enabled: bool) -> bool {
        let localized = translate(label);
        let width = crate::measure_text_guarded(&localized, Some(font), 22, 1.).width;
        let size = if width > 225. {
            (22. * 225. / width).max(12.) as u16
        } else {
            22
        };
        Self::label_raw(font, &localized, 160., y + 28., size, ORANGE);
        if label == "Player Name" || label == "Ball Model" {
            self.button_raw(font, Rect::new(400., y, 245., 38.), value, 24, enabled)
        } else {
            self.button(font, Rect::new(400., y, 245., 38.), value, 24, enabled)
        }
    }
    fn draw_options(&mut self, font: &Font, flow: &mut Flow, s: &mut Settings) {
        if self.options_page != OptionsPage::Ball {
            clear_background(Color::new(0.08, 0.28, 0.44, 1.));
        }
        let title = match self.options_page {
            OptionsPage::Main => "Options",
            OptionsPage::Graphics => "Graphics",
            OptionsPage::Configure => "Configure",
            OptionsPage::Bindings => "Key Bindings",
            OptionsPage::Language => "Language",
            OptionsPage::Player => "Player Name",
            OptionsPage::Ball => "Ball Model",
            OptionsPage::Resolution => "Resolution",
        };
        Self::centered(font, title, 75., 42, ORANGE);
        if self.options_page != OptionsPage::Player
            && self.binding_capture.is_none()
            && (self.button(font, Rect::new(80., 30., 120., 42.), "Back", 26, true)
                || is_key_pressed(KeyCode::Escape))
        {
            if self.options_page == OptionsPage::Main {
                flow.screen = Screen::Title;
            } else {
                self.options_page = OptionsPage::Main;
            }
            self.reset_focus();
            return;
        }
        match self.options_page {
            OptionsPage::Main => {
                if self.option_row(font, 110., "Graphics", "Configure", true) {
                    self.options_page = OptionsPage::Graphics;
                    self.reset_focus();
                }
                if self.option_row(font, 155., "Controls", "Configure", true) {
                    self.options_page = OptionsPage::Configure;
                    self.reset_focus();
                }
                if self.option_row(
                    font,
                    210.,
                    "Mouse Sensitivity",
                    &format!("{}", s.mouse_sense),
                    true,
                ) {
                    s.mouse_sense = if s.mouse_sense >= 600 {
                        100
                    } else {
                        s.mouse_sense + 50
                    };
                }
                if self.option_row(
                    font,
                    260.,
                    "Sound Volume",
                    &s.sound_volume.to_string(),
                    true,
                ) {
                    s.sound_volume = (s.sound_volume + 1) % 11;
                }
                if self.option_row(
                    font,
                    305.,
                    "Music Volume",
                    &s.music_volume.to_string(),
                    true,
                ) {
                    s.music_volume = (s.music_volume + 1) % 11;
                }
                if self.option_row(font, 370., "Player Name", &s.player, true) {
                    self.player_edit = s.player.clone();
                    self.options_page = OptionsPage::Player;
                    self.reset_focus();
                }
                if self.option_row(
                    font,
                    415.,
                    "Ball Model",
                    s.ball_file.rsplit('/').next().unwrap_or("basic-ball"),
                    true,
                ) {
                    self.options_page = OptionsPage::Ball;
                    self.reset_focus();
                }
                let label = LANGUAGES
                    .iter()
                    .find(|(code, _)| *code == s.language)
                    .map(|(_, label)| *label)
                    .unwrap_or("English");
                if self.option_row(font, 460., "Language", label, true) {
                    self.options_page = OptionsPage::Language;
                    self.reset_focus();
                }
            }
            OptionsPage::Graphics => {
                if self.option_row(
                    font,
                    110.,
                    "Fullscreen",
                    if s.fullscreen { "On" } else { "Off" },
                    true,
                ) {
                    s.fullscreen = !s.fullscreen;
                }
                if self.option_row(
                    font,
                    155.,
                    "Resolution",
                    &format!("{} x {}", s.width, s.height),
                    cfg!(not(target_arch = "wasm32")),
                ) {
                    self.options_page = OptionsPage::Resolution;
                    self.reset_focus();
                }
                if self.option_row(
                    font,
                    210.,
                    "Reflection",
                    if s.reflection { "On" } else { "Off" },
                    true,
                ) {
                    s.reflection = !s.reflection;
                }
                if self.option_row(
                    font,
                    255.,
                    "Background",
                    if s.background { "On" } else { "Off" },
                    true,
                ) {
                    s.background = !s.background;
                }
                if self.option_row(
                    font,
                    300.,
                    "Shadow",
                    if s.shadow { "On" } else { "Off" },
                    true,
                ) {
                    s.shadow = !s.shadow;
                }
                if self.option_row(
                    font,
                    345.,
                    "Textures",
                    if s.textures { "On" } else { "Off" },
                    true,
                ) {
                    s.textures = !s.textures;
                }
                if self.option_row(
                    font,
                    390.,
                    "Texture Filtering",
                    if s.mipmap { "Linear" } else { "Nearest" },
                    true,
                ) {
                    s.mipmap = !s.mipmap;
                }
                if self.option_row(
                    font,
                    435.,
                    "V-Sync",
                    if cfg!(target_arch = "wasm32") {
                        "Browser controlled"
                    } else if s.vsync {
                        "On (restart)"
                    } else {
                        "Off (restart)"
                    },
                    cfg!(not(target_arch = "wasm32")),
                ) {
                    s.vsync = !s.vsync;
                }
                let aa = if cfg!(target_arch = "wasm32") {
                    "Unavailable".to_string()
                } else {
                    format!("{}x (restart)", s.multisample.max(1))
                };
                if self.option_row(
                    font,
                    480.,
                    "Antialiasing",
                    &aa,
                    cfg!(not(target_arch = "wasm32")),
                ) {
                    s.multisample = match s.multisample {
                        0 | 1 => 2,
                        2 => 4,
                        4 => 8,
                        _ => 0,
                    };
                }
            }
            OptionsPage::Configure => {
                if self.option_row(
                    font,
                    125.,
                    "Camera",
                    ["Chase", "Lazy", "Manual"][(s.camera.saturating_sub(1) as usize).min(2)],
                    true,
                ) {
                    s.camera = s.camera % 3 + 1;
                }
                if self.option_row(
                    font,
                    175.,
                    "Mouse Invert",
                    if s.mouse_invert { "On" } else { "Off" },
                    true,
                ) {
                    s.mouse_invert = !s.mouse_invert;
                }
                if self.option_row(
                    font,
                    225.,
                    "Mouse Response",
                    &format!("{} ms", s.mouse_response),
                    true,
                ) {
                    s.mouse_response = (s.mouse_response + 25) % 275;
                }
                if self.option_row(
                    font,
                    275.,
                    "Key Response",
                    &format!("{} ms", s.joystick_response),
                    true,
                ) {
                    s.joystick_response = (s.joystick_response + 50) % 550;
                }
                if self.option_row(font, 340., "Key Bindings", "Configure", true) {
                    self.options_page = OptionsPage::Bindings;
                    self.reset_focus();
                }
                for (i, (action, label)) in [
                    ("mouse_camera_l", "Mouse rotate left"),
                    ("mouse_camera_r", "Mouse rotate right"),
                    ("mouse_camera_toggle", "Mouse camera"),
                ]
                .iter()
                .enumerate()
                {
                    let current = s
                        .bindings
                        .get(*action)
                        .cloned()
                        .unwrap_or_else(|| "left".into());
                    if self.option_row(font, 395. + i as f32 * 45., label, &current, true) {
                        let next = match current.as_str() {
                            "left" => "middle",
                            "middle" => "right",
                            _ => "left",
                        };
                        s.bindings.insert((*action).into(), next.into());
                    }
                }
                if self.option_row(
                    font,
                    530.,
                    "Gamepad",
                    if cfg!(target_arch = "wasm32") {
                        if s.joystick {
                            "On"
                        } else {
                            "Off"
                        }
                    } else {
                        "Browser only"
                    },
                    cfg!(target_arch = "wasm32"),
                ) {
                    s.joystick = !s.joystick;
                }
            }
            OptionsPage::Language => {
                for (i, (code, label)) in LANGUAGES.iter().enumerate() {
                    if self.button(
                        font,
                        Rect::new(
                            110. + (i % 2) as f32 * 300.,
                            115. + (i / 2) as f32 * 75.,
                            280.,
                            60.,
                        ),
                        label,
                        28,
                        *code == "en" || self.translations.contains_key(*code),
                    ) {
                        s.language = (*code).into();
                        self.options_page = OptionsPage::Main;
                        self.reset_focus();
                    }
                }
            }
            OptionsPage::Bindings => {
                if let Some(action) = self.binding_capture.clone() {
                    Self::centered(font, "Press a key. Escape cancels.", 240., 30, YELLOW);
                    Self::centered(font, &action, 295., 23, WHITE);
                    if get_time() - self.binding_since > 0.1 {
                        if is_key_pressed(KeyCode::Escape) {
                            self.binding_capture = None;
                        } else {
                            let mut keys: Vec<_> = get_keys_pressed()
                                .into_iter()
                                .map(|k| format!("{k:?}"))
                                .collect();
                            keys.sort();
                            if let Some(key) = keys.first() {
                                match s.set_key_binding(&action, key) {
                                    Ok(()) => {
                                        self.binding_capture = None;
                                        self.status.clear();
                                    }
                                    Err(e) => self.status = e,
                                }
                            }
                        }
                    }
                } else {
                    let entries = reagent_neverball_rs::settings::KEY_BINDINGS;
                    for (i, (action, label)) in entries
                        .iter()
                        .skip(self.binding_page * 7)
                        .take(7)
                        .enumerate()
                    {
                        if self.option_row(
                            font,
                            115. + i as f32 * 48.,
                            label,
                            &s.binding_name(action),
                            true,
                        ) {
                            self.binding_capture = Some((*action).into());
                            self.binding_since = get_time();
                        }
                    }
                    if self.button(
                        font,
                        Rect::new(150., 480., 170., 45.),
                        "Previous",
                        25,
                        self.binding_page > 0,
                    ) {
                        self.binding_page -= 1;
                    }
                    if self.button(
                        font,
                        Rect::new(480., 480., 170., 45.),
                        "Next",
                        25,
                        (self.binding_page + 1) * 7 < entries.len(),
                    ) {
                        self.binding_page += 1;
                    }
                    if self.button(
                        font,
                        Rect::new(300., 535., 200., 40.),
                        "Restore Defaults",
                        22,
                        true,
                    ) {
                        for (action, _) in entries {
                            s.bindings.insert(
                                (*action).into(),
                                Settings::default().bindings[*action].clone(),
                            );
                        }
                    }
                }
            }
            OptionsPage::Resolution => {
                for (i, (w, h)) in [
                    (640, 480),
                    (800, 600),
                    (1024, 768),
                    (1280, 720),
                    (1280, 960),
                    (1920, 1080),
                ]
                .iter()
                .enumerate()
                {
                    if self.button(
                        font,
                        Rect::new(240., 115. + i as f32 * 60., 320., 50.),
                        &format!("{w} x {h}"),
                        28,
                        true,
                    ) {
                        s.width = *w;
                        s.height = *h;
                        self.options_page = OptionsPage::Graphics;
                        self.reset_focus();
                    }
                }
            }
            OptionsPage::Player => {
                for c in typed_characters() {
                    if !c.is_control() && self.player_edit.chars().count() < 32 {
                        self.player_edit.push(c);
                    }
                }
                if is_key_pressed(KeyCode::Backspace) {
                    self.player_edit.pop();
                }
                Self::label_raw(
                    font,
                    &self.player_edit,
                    400. - crate::measure_text_guarded(&self.player_edit, Some(font), 38, 1.).width
                        / 2.,
                    180.,
                    38,
                    YELLOW,
                );
                for (i, c) in "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ".chars().enumerate() {
                    let c = if self.caps { c } else { c.to_ascii_lowercase() };
                    if self.button(
                        font,
                        Rect::new(
                            205. + (i % 10) as f32 * 39.,
                            230. + (i / 10) as f32 * 42.,
                            38.,
                            40.,
                        ),
                        &c.to_string(),
                        24,
                        true,
                    ) && self.player_edit.chars().count() < 32
                    {
                        self.player_edit.push(c);
                    }
                }
                if self.button(font, Rect::new(200., 405., 120., 40.), "Caps", 24, true) {
                    self.caps = !self.caps;
                }
                if self.button(font, Rect::new(340., 405., 120., 40.), "Space", 24, true)
                    && self.player_edit.chars().count() < 32
                {
                    self.player_edit.push(' ');
                }
                if self.button(font, Rect::new(480., 405., 120., 40.), "<", 24, true) {
                    self.player_edit.pop();
                }
                if self.button(font, Rect::new(200., 475., 160., 45.), "Cancel", 26, true)
                    || is_key_pressed(KeyCode::Escape)
                {
                    self.options_page = OptionsPage::Main;
                    self.reset_focus();
                }
                if self.button(
                    font,
                    Rect::new(440., 475., 160., 45.),
                    "OK",
                    26,
                    !self.player_edit.trim().is_empty(),
                ) {
                    s.player = self.player_edit.trim().to_string();
                    self.options_page = OptionsPage::Main;
                    self.reset_focus();
                }
            }
            OptionsPage::Ball => {
                let name = s.ball_file.rsplit('/').next().unwrap_or("basic-ball");
                let index = BALLS.iter().position(|&v| v == name).unwrap_or(1);
                Self::centered(font, name, 150., 30, WHITE);
                let left = self.button(font, Rect::new(180., 115., 55., 50.), "<", 30, true)
                    || is_key_pressed(KeyCode::Left);
                let right = self.button(font, Rect::new(565., 115., 55., 50.), ">", 30, true)
                    || is_key_pressed(KeyCode::Right);
                if left || right {
                    let next = if right {
                        (index + 1) % BALLS.len()
                    } else {
                        (index + BALLS.len() - 1) % BALLS.len()
                    };
                    let name = BALLS[next];
                    s.ball_file = format!("ball/{name}/{name}");
                }
            }
        }
    }
}

impl Ui {
    pub fn draw_replay_end(&mut self, font: &Font) -> ReplayEndAction {
        set_camera(&Camera2D {
            target: vec2(400., 300.),
            zoom: vec2(2. / 800., 2. / 600.),
            ..Default::default()
        });
        self.button_index = 0;
        Self::panel(
            Rect::new(100., 225., 600., 130.),
            Color::new(0., 0., 0., 0.6),
        );
        Self::centered(font, "Replay Ends", 315., 72, RED);
        let mut action = ReplayEndAction::None;
        if self.button(font, Rect::new(100., 360., 195., 50.), "Repeat", 30, true) {
            action = ReplayEndAction::Repeat;
        }
        if self.button(font, Rect::new(302., 360., 195., 50.), "Delete", 30, true) {
            action = ReplayEndAction::Delete;
        }
        if self.button(font, Rect::new(505., 360., 195., 50.), "Keep", 30, true) {
            action = ReplayEndAction::Keep;
        }
        set_default_camera();
        action
    }
}

// Macroquad stores character events as a stack; restore arrival order each frame.
fn typed_characters() -> impl Iterator<Item = char> {
    let mut chars = Vec::new();
    while let Some(c) = get_char_pressed() {
        chars.push(c);
    }
    chars.into_iter().rev()
}

fn binding_pressed(settings: &Settings, action: &str) -> bool {
    let name = settings.binding_name(action);
    get_keys_pressed()
        .into_iter()
        .any(|key| format!("{key:?}") == name)
}

const LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("de", "Deutsch"),
    ("es", "Español"),
    ("fr", "Français"),
    ("it", "Italiano"),
    ("pt_BR", "Português (Brasil)"),
    ("nl", "Nederlands"),
    ("pl", "Polski"),
    ("ru", "Русский"),
];
thread_local! {static TRANSLATIONS:std::cell::RefCell<BTreeMap<String,String>>=const {std::cell::RefCell::new(BTreeMap::new())};}
pub fn translate(text: &str) -> String {
    TRANSLATIONS.with(|t| t.borrow().get(text).cloned().unwrap_or_else(|| text.into()))
}
