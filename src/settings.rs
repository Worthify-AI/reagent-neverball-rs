// SPDX-License-Identifier: GPL-3.0-or-later
//! Runtime configuration, using values retained from the reference installation.
//! Platform/backend capability is separate from a setting's stored value.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub player: String,
    pub language: String,
    pub camera: u8,
    pub sound_volume: u8,
    pub music_volume: u8,
    pub fullscreen: bool,
    pub width: u32,
    pub height: u32,
    pub vsync: bool,
    pub textures: bool,
    pub reflection: bool,
    pub background: bool,
    pub shadow: bool,
    pub mipmap: bool,
    pub aniso: u32,
    pub multisample: u32,
    pub mouse_sense: u32,
    pub mouse_response: u32,
    pub mouse_invert: bool,
    pub joystick: bool,
    pub joystick_response: u32,
    pub view_fov: u32,
    pub rotate_fast: u32,
    pub rotate_slow: u32,
    pub lock_goals: bool,
    pub ball_file: String,
    pub replay_name: String,
    pub bindings: BTreeMap<String, String>,
    pub extra: BTreeMap<String, String>,
}
impl Default for Settings {
    fn default() -> Self {
        let bindings = [
            ("key_camera_1", "1"),
            ("key_camera_2", "2"),
            ("key_camera_3", "3"),
            ("key_camera_toggle", "E"),
            ("key_camera_r", "D"),
            ("key_camera_l", "S"),
            ("key_forward", "Up"),
            ("key_backward", "Down"),
            ("key_left", "Left"),
            ("key_right", "Right"),
            ("key_restart", "R"),
            ("key_score_next", "Tab"),
            ("key_rotate_fast", "Left Shift"),
            ("mouse_camera_toggle", "middle"),
            ("mouse_camera_l", "left"),
            ("mouse_camera_r", "right"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect();
        Self {
            player: "Player".into(),
            language: "en".into(),
            camera: 1,
            sound_volume: 10,
            music_volume: 6,
            fullscreen: false,
            width: 800,
            height: 600,
            vsync: true,
            textures: true,
            reflection: true,
            background: true,
            shadow: true,
            mipmap: true,
            aniso: 8,
            multisample: 0,
            mouse_sense: 300,
            mouse_response: 50,
            mouse_invert: false,
            joystick: true,
            joystick_response: 250,
            view_fov: 50,
            rotate_fast: 300,
            rotate_slow: 150,
            lock_goals: true,
            ball_file: "ball/basic-ball/basic-ball".into(),
            replay_name: "%s-%l".into(),
            bindings,
            extra: BTreeMap::new(),
        }
    }
}
impl Settings {
    pub fn sound_gain(&self) -> f32 {
        self.sound_volume.min(10) as f32 / 10.
    }
    pub fn music_gain(&self) -> f32 {
        self.music_volume.min(10) as f32 / 10.
    }
    pub fn check(&self) -> Result<(), String> {
        if !(1..=3).contains(&self.camera) {
            return Err("Camera must be 1, 2, or 3".into());
        }
        if self.sound_volume > 10 || self.music_volume > 10 {
            return Err("Volume must be 0 through 10".into());
        }
        if self.width == 0
            || self.height == 0
            || self.mouse_sense == 0
            || self.view_fov == 0
            || self.view_fov >= 180
        {
            return Err("Invalid display or input setting".into());
        }
        Ok(())
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let out: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        out.check()?;
        Ok(out)
    }
    /// Parse the observed whitespace-delimited rc file, preserving unknown keys.
    pub fn from_rc(text: &str) -> Result<Self, String> {
        let mut out = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once(char::is_whitespace)
                .map(|(k, v)| (k, v.trim()))
                .unwrap_or((line, ""));
            macro_rules! num {
                ($field:ident) => {{
                    out.$field = value.parse().map_err(|_| format!("Invalid {}", key))?;
                }};
            }
            macro_rules! flag {
                ($field:ident) => {{
                    out.$field = match value {
                        "0" => false,
                        "1" => true,
                        _ => return Err(format!("Invalid {}", key)),
                    };
                }};
            }
            match key {
                "player" => out.player = value.into(),
                "language" => out.language = value.into(),
                "camera" => {
                    let raw: u8 = value.parse().map_err(|_| "Invalid camera")?;
                    if raw > 2 {
                        return Err("Invalid camera".into());
                    }
                    out.camera = raw + 1;
                }
                "sound_volume" => num!(sound_volume),
                "music_volume" => num!(music_volume),
                "fullscreen" => flag!(fullscreen),
                "width" => num!(width),
                "height" => num!(height),
                "vsync" => flag!(vsync),
                "textures" => flag!(textures),
                "reflection" => flag!(reflection),
                "background" => flag!(background),
                "shadow" => flag!(shadow),
                "mipmap" => flag!(mipmap),
                "aniso" => num!(aniso),
                "multisample" => num!(multisample),
                "mouse_sense" => num!(mouse_sense),
                "mouse_response" => num!(mouse_response),
                "mouse_invert" => flag!(mouse_invert),
                "joystick" => flag!(joystick),
                "joystick_response" => num!(joystick_response),
                "view_fov" => num!(view_fov),
                "rotate_fast" => num!(rotate_fast),
                "rotate_slow" => num!(rotate_slow),
                "lock_goals" => flag!(lock_goals),
                "ball_file" => out.ball_file = value.into(),
                "replay_name" => out.replay_name = value.into(),
                k if k.starts_with("key_") || k.starts_with("mouse_camera_") => {
                    out.bindings.insert(k.into(), value.into());
                }
                _ => {
                    out.extra.insert(key.into(), value.into());
                }
            }
        }
        out.check()?;
        Ok(out)
    }
    pub fn to_rc(&self) -> String {
        let mut values = self.extra.clone();
        values.extend(self.bindings.clone());
        macro_rules! fields {($($field:ident),*)=>{$(values.insert(stringify!($field).into(),self.$field.to_string());)*}}
        macro_rules! flags {($($field:ident),*)=>{$(values.insert(stringify!($field).into(),u8::from(self.$field).to_string());)*}}
        fields!(
            player,
            language,
            sound_volume,
            music_volume,
            width,
            height,
            aniso,
            multisample,
            mouse_sense,
            mouse_response,
            joystick_response,
            view_fov,
            rotate_fast,
            rotate_slow,
            ball_file,
            replay_name
        );
        values.insert("camera".into(), self.camera.saturating_sub(1).to_string());
        flags!(
            fullscreen,
            vsync,
            textures,
            reflection,
            background,
            shadow,
            mipmap,
            mouse_invert,
            joystick,
            lock_goals
        );
        values
            .into_iter()
            .map(|(k, v)| format!("{k:<26}{v}\n"))
            .collect()
    }
}

/// Host persists this one versioned document. Browser hosts can store its JSON
/// in localStorage; native hosts use the atomic file helpers below.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub version: u32,
    pub settings: Settings,
    pub progress: crate::flow::Progress,
    pub replays: crate::replay::ReplayStore,
}
impl SaveData {
    pub fn new() -> Self {
        Self {
            version: 1,
            ..Self::default()
        }
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let mut save: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if save.version > 1 {
            return Err("Unsupported save version".into());
        }
        save.version = 1;
        save.settings.check()?;
        crate::replay::ReplayStore::from_json(&save.replays.to_json().map_err(|e| e.to_string())?)?;
        Ok(save)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        match std::fs::read_to_string(path) {
            Ok(s) => Self::from_json(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(e.to_string()),
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, path: &std::path::Path) -> Result<(), String> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("json.tmp");
        use std::io::Write;
        let mut file = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        file.write_all(self.to_json().map_err(|e| e.to_string())?.as_bytes())
            .map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        std::fs::rename(tmp, path).map_err(|e| e.to_string())
    }
}

/// Actions present in the reference configuration, with user-facing labels.
pub const KEY_BINDINGS: &[(&str, &str)] = &[
    ("key_forward", "Forward"),
    ("key_backward", "Backward"),
    ("key_left", "Left"),
    ("key_right", "Right"),
    ("key_camera_1", "Chase camera"),
    ("key_camera_2", "Lazy camera"),
    ("key_camera_3", "Manual camera"),
    ("key_camera_toggle", "Toggle camera"),
    ("key_camera_l", "Rotate left"),
    ("key_camera_r", "Rotate right"),
    ("key_rotate_fast", "Fast rotation"),
    ("key_restart", "Restart"),
    ("key_score_next", "Next score table"),
];
impl Settings {
    /// Canonical Macroquad key names; accepts retained native rc aliases too.
    pub fn binding_name(&self, action: &str) -> String {
        let value = self.bindings.get(action).map(String::as_str).unwrap_or("");
        canonical_key_name(value)
    }
    pub fn set_key_binding(&mut self, action: &str, key: &str) -> Result<(), String> {
        if !KEY_BINDINGS.iter().any(|(name, _)| *name == action) {
            return Err("Unknown key action".into());
        }
        let key = canonical_key_name(key);
        if key.is_empty() || key == "Unknown" || key == "Escape" {
            return Err("Choose another key; Escape cancels editing".into());
        }
        self.bindings.insert(action.into(), key);
        Ok(())
    }
}
pub fn canonical_key_name(value: &str) -> String {
    let compact = value.replace(" ", "");
    match compact.as_str() {
        "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" => format!("Key{compact}"),
        "Return" => "Enter".into(),
        "PageUp" | "PageDown" | "LeftShift" | "RightShift" | "LeftControl" | "RightControl"
        | "LeftAlt" | "RightAlt" => compact,
        _ if compact.len() == 1 => compact.to_ascii_uppercase(),
        _ => compact,
    }
}
