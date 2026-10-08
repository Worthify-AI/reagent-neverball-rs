// SPDX-License-Identifier: GPL-3.0-or-later
//! Licensed runtime sounds, selected from binary-derived game event calls.
use macroquad::audio::{
    load_sound, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound,
};
use reagent_neverball_rs::{
    content::safe_path,
    entities::{Event, FullGame},
};
use std::collections::BTreeMap;

pub struct SoundBank {
    effects: BTreeMap<&'static str, Sound>,
    music: Option<Sound>,
    music_path: String,
    data_root: String,
    pub sound_volume: f32,
    pub music_volume: f32,
    music_started: bool,
}
impl SoundBank {
    pub async fn load(data_root: &str) -> Result<Self, macroquad::Error> {
        let mut effects = BTreeMap::new();
        for name in [
            "coin", "grow", "shrink", "switch", "jump", "goal", "fall", "time", "bump", "bumpbig",
            "bumplil", "menu", "select", "ready", "set", "go", "success", "over", "record",
        ] {
            let sound = load_sound(&sound_path(data_root, &format!("snd/{name}.ogg"))).await?;
            effects.insert(name, sound);
        }
        Ok(Self {
            effects,
            music: None,
            music_path: String::new(),
            data_root: data_root.into(),
            sound_volume: 1.,
            music_volume: 0.6,
            music_started: false,
        })
    }
    pub async fn set_song(&mut self, path: &str) -> Result<(), macroquad::Error> {
        if self.music_path == path {
            return Ok(());
        }
        if let Some(sound) = &self.music {
            stop_sound(sound);
        }
        self.music_started = false;
        self.music = None;
        self.music_path = path.into();
        if safe_path(path) {
            self.music = Some(load_sound(&sound_path(&self.data_root, path)).await?);
        }
        Ok(())
    }
    pub fn volumes(&mut self, sound: f32, music: f32) {
        self.sound_volume = sound.clamp(0., 1.);
        self.music_volume = music.clamp(0., 1.);
        if let Some(song) = &self.music {
            set_sound_volume(song, self.music_volume);
        }
    }
    /// Call after a user input on the web, so the browser can resume audio.
    pub fn start_music(&mut self) {
        if self.music_started {
            return;
        }
        if let Some(sound) = &self.music {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: true,
                    volume: self.music_volume,
                },
            );
            self.music_started = true;
        }
    }
    pub fn mute_music(&mut self, muted: bool) {
        if let Some(sound) = &self.music {
            set_sound_volume(sound, if muted { 0. } else { self.music_volume });
        }
    }
    pub fn stop_music(&mut self) {
        if let Some(sound) = &self.music {
            stop_sound(sound);
        }
        self.music_started = false;
    }
    pub fn cue(&self, name: &str, level: f32) {
        if let Some(sound) = self.effects.get(name) {
            play_sound(
                sound,
                PlaySoundParams {
                    looped: false,
                    volume: (level * self.sound_volume).clamp(0., 1.),
                },
            );
        }
    }
    pub fn events(&self, game: &FullGame) {
        let radius = game.radius();
        let base_radius = game.base_radius;
        for event in &game.events {
            match event {
                Event::Item(index) => {
                    if let Some(item) = game.sol.items.get(*index) {
                        match item.kind {
                            1 => self.cue("coin", 1.),
                            2 => self.cue("grow", 1.),
                            3 => self.cue("shrink", 1.),
                            _ => {}
                        }
                    }
                }
                Event::Switch(_, _) => self.cue("switch", 1.),
                Event::JumpStarted => self.cue("jump", 1.),
                Event::Won => self.cue("goal", 1.),
                Event::Fell => self.cue("fall", 1.),
                Event::TimedOut => self.cue("time", 1.),
                // FUN_1abc0 emits bump for speed>0.5, with volume 2*(speed-0.5).
                Event::Impact(speed) => self.cue(
                    if radius > base_radius {
                        "bumpbig"
                    } else if radius < base_radius {
                        "bumplil"
                    } else {
                        "bump"
                    },
                    2. * (speed - 0.5),
                ),
                _ => {}
            }
        }
    }
}

// Safari Web Audio does not consistently decode the reference Ogg files.
// Browser-only MP3 derivatives retain their original license and provenance.
fn sound_path(data_root: &str, path: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    if let Some(stem) = path.strip_suffix(".ogg") {
        return format!("{data_root}/web-audio/{stem}.mp3");
    }
    format!("{data_root}/{path}")
}
