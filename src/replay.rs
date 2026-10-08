// SPDX-License-Identifier: GPL-3.0-or-later
//! Versioned reconstruction replay container. The reference has named replays,
//! Last, pause, playback and deletion (FUN_203b0, 25be0, 26ee0, 27110).
//! JSON saves use our versioned format; OriginalReplay imports original v9 NBR.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub const FORMAT: &str = "neverball-rust-replay";
pub const VERSION: u32 = 1;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    pub at_ms: u32,
    pub position: [f32; 3],
    pub orientation: [[f32; 3]; 3],
    pub radius: f32,
    pub camera_eye: [f32; 3],
    pub camera_target: [f32; 3],
    pub tilt: [f32; 2],
    pub coins: i32,
    pub remaining_seconds: f32,
    /// Collected items, switches, moving-body transforms, and other renderer state.
    #[serde(default)]
    pub state: serde_json::Value,
}
impl Frame {
    fn finite(&self) -> bool {
        self.position
            .iter()
            .chain(self.orientation.iter().flatten())
            .chain(self.camera_eye.iter())
            .chain(self.camera_target.iter())
            .chain(self.tilt.iter())
            .all(|x| x.is_finite())
            && self.radius.is_finite()
            && self.radius > 0.
            && self.remaining_seconds.is_finite()
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Replay {
    pub format: String,
    pub version: u32,
    pub set_id: String,
    pub level_id: String,
    pub player: String,
    pub outcome: Option<crate::flow::Outcome>,
    pub frames: Vec<Frame>,
}
impl Replay {
    pub fn new(set_id: &str, level_id: &str, player: &str) -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            set_id: set_id.into(),
            level_id: level_id.into(),
            player: player.into(),
            outcome: None,
            frames: vec![],
        }
    }
    pub fn push(&mut self, frame: Frame) -> Result<(), String> {
        if !frame.finite() {
            return Err("Invalid replay coordinates".into());
        }
        if self
            .frames
            .last()
            .is_some_and(|last| frame.at_ms <= last.at_ms)
        {
            return Err("Replay time must increase".into());
        }
        self.frames.push(frame);
        Ok(())
    }
    pub fn duration_ms(&self) -> u32 {
        self.frames.last().map_or(0, |f| f.at_ms)
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let replay: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if replay.format != FORMAT || replay.version != VERSION {
            return Err("Unsupported replay format/version".into());
        }
        if replay.frames.iter().any(|f| !f.finite())
            || replay.frames.windows(2).any(|p| p[0].at_ms >= p[1].at_ms)
        {
            return Err("Invalid replay frames".into());
        }
        Ok(replay)
    }
    /// Exact recorded states, selected by timestamp: playback does not rerun physics.
    pub fn frame_at(&self, at_ms: u32) -> Option<&Frame> {
        if self.frames.is_empty() {
            return None;
        }
        let i = self
            .frames
            .partition_point(|f| f.at_ms <= at_ms)
            .saturating_sub(1);
        self.frames.get(i)
    }
}
#[derive(Clone, Debug)]
pub struct Playback {
    pub replay: Replay,
    pub at_ms: f64,
    pub paused: bool,
    pub ended: bool,
    pub speed: f64,
}
impl Playback {
    pub fn new(replay: Replay) -> Self {
        Self {
            replay,
            at_ms: 0.,
            paused: false,
            ended: false,
            speed: 1.,
        }
    }
    pub fn advance(&mut self, dt_seconds: f64) -> Option<&Frame> {
        if !self.paused
            && !self.ended
            && dt_seconds.is_finite()
            && dt_seconds > 0.
            && self.speed.is_finite()
            && self.speed > 0.
        {
            self.at_ms = (self.at_ms + dt_seconds * 1000. * self.speed)
                .min(self.replay.duration_ms() as f64);
            self.ended = self.at_ms >= self.replay.duration_ms() as f64;
        }
        self.replay.frame_at(self.at_ms as u32)
    }
    pub fn seek(&mut self, at_ms: u32) {
        self.at_ms = at_ms.min(self.replay.duration_ms()) as f64;
        self.ended = self.at_ms >= self.replay.duration_ms() as f64;
    }
    pub fn restart(&mut self) {
        self.at_ms = 0.;
        self.ended = false;
        self.paused = false;
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReplayStore {
    pub last: Option<Replay>,
    pub saved: BTreeMap<String, Replay>,
}
impl ReplayStore {
    pub fn set_last(&mut self, replay: Replay) {
        self.last = Some(replay);
    }
    /// Logical names never become filesystem paths in this portable store.
    pub fn save_last(&mut self, name: &str, overwrite: bool) -> Result<(), String> {
        if name.trim().is_empty() || name.len() > 255 || name.chars().any(char::is_control) {
            return Err("Invalid replay name".into());
        }
        if self.saved.contains_key(name) && !overwrite {
            return Err("Replay already exists".into());
        }
        let replay = self.last.as_ref().ok_or("No last replay")?.clone();
        self.saved.insert(name.into(), replay);
        Ok(())
    }
    pub fn delete(&mut self, name: &str) -> bool {
        self.saved.remove(name).is_some()
    }
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
    pub fn from_json(text: &str) -> Result<Self, String> {
        let store: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        for replay in store.last.iter().chain(store.saved.values()) {
            Replay::from_json(&replay.to_json().map_err(|e| e.to_string())?)?;
        }
        Ok(store)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NbrHeader {
    pub version: u32,
    pub elapsed_centiseconds: i32,
    pub coins: i32,
    pub status: i32,
    pub mode: i32,
    pub player: String,
    pub date: String,
    pub screenshot: String,
    pub level: String,
    pub initial: [i32; 6],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NbrCommand {
    pub kind: u8,
    pub payload: Vec<u8>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NbrBatch {
    pub at_seconds: f64,
    pub commands: Vec<NbrCommand>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OriginalReplay {
    pub header: NbrHeader,
    pub batches: Vec<NbrBatch>,
    pub replay: Replay,
    pub unhandled_kinds: Vec<u8>,
}
impl OriginalReplay {
    /// Import observed v9 NBR command streams. Command payloads are retained in
    /// full; unhandled kinds are explicit instead of silently asserting parity.
    /// Framing/header observed in captures; camera IDs also independently used
    /// by the reference-camera trace parser. Dynamic command schemas are from
    /// generated FUN_1e9f0 (decoder), FUN_1dbe0 (application), FUN_33d00.
    /// No upstream implementation read.
    pub fn from_nbr(bytes: &[u8]) -> Result<Self, String> {
        struct Reader<'a> {
            b: &'a [u8],
            o: usize,
        }
        impl<'a> Reader<'a> {
            fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
                let end = self.o.checked_add(n).ok_or("NBR length overflow")?;
                let out = self.b.get(self.o..end).ok_or("Truncated NBR")?;
                self.o = end;
                Ok(out)
            }
            fn int(&mut self) -> Result<i32, String> {
                Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
            }
            fn string(&mut self) -> Result<String, String> {
                let n = self.b[self.o..]
                    .iter()
                    .position(|&x| x == 0)
                    .ok_or("Unterminated NBR string")?;
                let s =
                    String::from_utf8(self.take(n)?.to_vec()).map_err(|_| "Invalid NBR text")?;
                self.take(1)?;
                Ok(s)
            }
        }
        fn floats<const N: usize>(p: &[u8]) -> Result<[f32; N], String> {
            if p.len() != N * 4 {
                return Err("Unexpected NBR command size".into());
            }
            let mut out = [0.; N];
            for (i, c) in p.chunks_exact(4).enumerate() {
                out[i] = f32::from_le_bytes(c.try_into().unwrap());
                if !out[i].is_finite() {
                    return Err("Non-finite NBR value".into());
                }
            }
            Ok(out)
        }
        fn int(p: &[u8]) -> Result<i32, String> {
            if p.len() != 4 {
                return Err("Unexpected NBR integer size".into());
            }
            Ok(i32::from_le_bytes(p.try_into().unwrap()))
        }
        fn basis(p: &[u8]) -> Result<[[f32; 3]; 3], String> {
            let v = floats::<6>(p)?;
            let a = [v[0], v[1], v[2]];
            let b = [v[3], v[4], v[5]];
            Ok([
                a,
                b,
                [
                    a[1] * b[2] - a[2] * b[1],
                    a[2] * b[0] - a[0] * b[2],
                    a[0] * b[1] - a[1] * b[0],
                ],
            ])
        }
        let mut rd = Reader { b: bytes, o: 0 };
        if rd.take(4)? != [0xaf, b'N', b'B', b'R'] {
            return Err("Not an NBR replay".into());
        }
        let version = rd.int()? as u32;
        if version != 9 {
            return Err(format!("Unsupported NBR version {version}"));
        }
        let elapsed_centiseconds = rd.int()?;
        let coins = rd.int()?;
        let status = rd.int()?;
        let mode = rd.int()?;
        let player = rd.string()?;
        let date = rd.string()?;
        let screenshot = rd.string()?;
        let level = rd.string()?;
        let mut initial = [0; 6];
        for n in &mut initial {
            *n = rd.int()?;
        }
        let header = NbrHeader {
            version,
            elapsed_centiseconds,
            coins,
            status,
            mode,
            player,
            date,
            screenshot,
            level,
            initial,
        };
        let mut replay = Replay::new("", &header.level, &header.player);
        replay.outcome = match status {
            1 => Some(crate::flow::Outcome::TimeOut),
            2 => Some(crate::flow::Outcome::Success),
            3 => Some(crate::flow::Outcome::FallOut),
            _ => None,
        };
        let mut frame = Frame {
            at_ms: 0,
            position: [0.; 3],
            orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            radius: 0.25,
            camera_eye: [0.; 3],
            camera_target: [0.; 3],
            tilt: [0.; 2],
            coins: 0,
            remaining_seconds: initial[0] as f32 / 100.,
            state: serde_json::json!({}),
        };
        let mut state = serde_json::Map::new();
        let mut items = Vec::<serde_json::Value>::new();
        let mut unknown = std::collections::BTreeSet::new();
        let mut batches = vec![];
        let mut commands = vec![];
        let mut t = 0.;
        let mut updates_per_second = None;
        while rd.o < bytes.len() {
            let kind = rd.take(1)?[0];
            let n = u16::from_le_bytes(rd.take(2)?.try_into().unwrap()) as usize;
            let payload = rd.take(n)?.to_vec();
            let p = payload.as_slice();
            match kind {
                1 => {
                    if n != 0 {
                        return Err("Invalid NBR batch marker".into());
                    }
                    let hz = updates_per_second.ok_or("NBR batch has no update rate")?;
                    if !batches.is_empty() {
                        t += 1. / hz as f64;
                    }
                    frame.at_ms = (t * 1000_f64).round() as u32;
                    state.insert("items".into(), serde_json::json!(items));
                    state.insert(
                        "collected".into(),
                        serde_json::json!(items
                            .iter()
                            .map(|v| v["collected"].as_bool().unwrap_or(false))
                            .collect::<Vec<_>>()),
                    );
                    frame.state = serde_json::Value::Object(state.clone());
                    replay.push(frame.clone())?;
                }
                2 => {
                    state.insert("ball_created".into(), true.into());
                }
                3 => {
                    if n != 20 {
                        return Err("Invalid item command".into());
                    }
                    let pos = floats::<3>(&p[..12])?;
                    items.push(serde_json::json!({"position":pos,"type":int(&p[12..16])?,"value":int(&p[16..20])?,"collected":false}));
                }
                4 => {
                    let index = int(p)?;
                    if let Some(item) = usize::try_from(index).ok().and_then(|i| items.get_mut(i)) {
                        item["collected"] = true.into();
                    }
                }
                5 => frame.tilt = floats::<2>(p)?,
                6 => {
                    let end = p
                        .iter()
                        .position(|&b| b == 0)
                        .ok_or("Invalid sound command")?;
                    let name = std::str::from_utf8(&p[..end]).map_err(|_| "Invalid sound name")?;
                    let gain = floats::<1>(&p[end + 1..])?[0];
                    state.insert(
                        "last_sound".into(),
                        serde_json::json!({"name":name,"gain":gain}),
                    );
                }
                7 => frame.remaining_seconds = floats::<1>(p)?[0],
                8 => {
                    state.insert("outcome".into(), int(p)?.into());
                }
                9 => frame.coins = int(p)?,
                10 | 11 => {
                    if n != 0 {
                        return Err("Invalid jump command".into());
                    }
                    state.insert("jump_enabled".into(), (kind == 11).into());
                }
                12 | 29 | 33 => {
                    if n != 8 {
                        return Err("Invalid path command".into());
                    }
                    let field = match kind {
                        12 => "body_paths",
                        29 => "path_flags",
                        _ => "mover_paths",
                    };
                    let object = state
                        .entry(field)
                        .or_insert_with(|| serde_json::json!({}))
                        .as_object_mut()
                        .unwrap();
                    object.insert(int(&p[..4])?.to_string(), int(&p[4..])?.into());
                }
                13 | 34 => {
                    if n != 8 {
                        return Err("Invalid mover time command".into());
                    }
                    let field = if kind == 13 {
                        "body_times"
                    } else {
                        "mover_times"
                    };
                    let object = state
                        .entry(field)
                        .or_insert_with(|| serde_json::json!({}))
                        .as_object_mut()
                        .unwrap();
                    object.insert(
                        int(&p[..4])?.to_string(),
                        serde_json::json!(floats::<1>(&p[4..])?[0]),
                    );
                }
                14 => {
                    if n != 0 {
                        return Err("Invalid goal command".into());
                    }
                    state.insert("goal_open".into(), true.into());
                }
                15 | 17 => {
                    let object = state
                        .entry("switch_entered")
                        .or_insert_with(|| serde_json::json!({}))
                        .as_object_mut()
                        .unwrap();
                    object.insert(int(p)?.to_string(), (kind == 15).into());
                }
                16 => {
                    let object = state
                        .entry("switch_toggle_count")
                        .or_insert_with(|| serde_json::json!({}))
                        .as_object_mut()
                        .unwrap();
                    let key = int(p)?.to_string();
                    let count = object.get(&key).and_then(|v| v.as_u64()).unwrap_or(0);
                    object.insert(key, (count + 1).into());
                }
                18 => {
                    let hz = int(p)?;
                    if !(1..=1000).contains(&hz) {
                        return Err("Unsupported NBR update rate".into());
                    }
                    updates_per_second = Some(hz);
                    state.insert("updates_per_second".into(), hz.into());
                }
                19 => frame.radius = floats::<1>(p)?[0],
                20 => items.clear(),
                21 => {
                    state.insert("balls_cleared".into(), true.into());
                }
                22 => frame.position = floats::<3>(p)?,
                23 => frame.orientation = basis(p)?,
                24 => {
                    state.insert("ball_pendulum_basis".into(), serde_json::json!(basis(p)?));
                }
                25 => frame.camera_eye = floats::<3>(p)?,
                26 => frame.camera_target = floats::<3>(p)?,
                27 => {
                    state.insert("camera_basis".into(), serde_json::json!(basis(p)?));
                }
                28 => {
                    state.insert("current_ball".into(), int(p)?.into());
                }
                30 => {
                    let dt = floats::<1>(p)?[0];
                    if dt < 0. {
                        return Err("Negative NBR time step".into());
                    }
                    // cmd30 advances movers/physics, not the replay clock.
                    // Jump batches still advance camera and replay time without it.
                }
                31 => {
                    state.insert("map_command".into(), serde_json::json!(payload));
                }
                32 => {
                    state.insert("tilt_basis".into(), serde_json::json!(floats::<6>(p)?));
                }
                _ => {
                    unknown.insert(kind);
                }
            }
            commands.push(NbrCommand { kind, payload });
            if kind == 1 {
                batches.push(NbrBatch {
                    at_seconds: t,
                    commands: std::mem::take(&mut commands),
                });
            }
        }
        if !commands.is_empty() {
            return Err("NBR ended before batch commit".into());
        }
        Ok(Self {
            header,
            batches,
            replay,
            unhandled_kinds: unknown.into_iter().collect(),
        })
    }
}
impl OriginalReplay {
    /// Lossless re-save of an imported command stream, including unknown kinds.
    pub fn to_nbr(&self) -> Result<Vec<u8>, String> {
        let mut out = vec![0xaf, b'N', b'B', b'R'];
        for n in [
            self.header.version as i32,
            self.header.elapsed_centiseconds,
            self.header.coins,
            self.header.status,
            self.header.mode,
        ] {
            out.extend(n.to_le_bytes());
        }
        for s in [
            &self.header.player,
            &self.header.date,
            &self.header.screenshot,
            &self.header.level,
        ] {
            if s.as_bytes().contains(&0) {
                return Err("NUL in NBR header string".into());
            }
            out.extend(s.as_bytes());
            out.push(0);
        }
        for n in self.header.initial {
            out.extend(n.to_le_bytes());
        }
        for batch in &self.batches {
            for cmd in &batch.commands {
                let n = u16::try_from(cmd.payload.len()).map_err(|_| "NBR command too long")?;
                out.push(cmd.kind);
                out.extend(n.to_le_bytes());
                out.extend(&cmd.payload);
            }
        }
        Ok(out)
    }
}
impl OriginalReplay {
    /// Resolve runtime mover initialization from the compiled SOL and apply the
    /// recorded clock/path commands. Unlike simulation, a replay never follows
    /// links automatically: FUN_33d00 advances enabled clocks on cmd30, while
    /// cmd33/cmd34 explicitly change path/reset elapsed.
    pub fn with_sol(mut self, sol: &crate::sol::Sol) -> Result<Self, String> {
        let mut paths = crate::entities::PathRuntime::new(sol);
        let mut switches: Vec<(bool, bool)> = sol
            .switches
            .iter()
            .map(|s| (s.words[0] != 0, false))
            .collect();
        fn integer(b: &[u8], offset: usize) -> Result<i32, String> {
            let a = b.get(offset..offset + 4).ok_or("Short replay command")?;
            Ok(i32::from_le_bytes(a.try_into().unwrap()))
        }
        fn float(b: &[u8], offset: usize) -> Result<f32, String> {
            Ok(f32::from_bits(integer(b, offset)? as u32))
        }
        for batch in &self.batches {
            for cmd in &batch.commands {
                let p = &cmd.payload;
                match cmd.kind {
                    12 | 13 | 33 | 34 => {
                        let index = integer(p, 0)?;
                        let index = if cmd.kind == 12 || cmd.kind == 13 {
                            usize::try_from(index)
                                .ok()
                                .and_then(|i| paths.body_movers.get(i))
                                .and_then(|p| p[0])
                        } else {
                            usize::try_from(index).ok()
                        };
                        if let Some(m) = index.and_then(|i| paths.movers.get_mut(i)) {
                            if cmd.kind == 12 || cmd.kind == 33 {
                                let next = integer(p, 4)?;
                                if next >= 0 && (next as usize) < sol.paths.len() {
                                    m.path = next as usize;
                                }
                            } else {
                                m.elapsed = float(p, 4)?;
                            }
                        }
                    }
                    15..=17 => {
                        if let Some(s) = usize::try_from(integer(p, 0)?)
                            .ok()
                            .and_then(|i| switches.get_mut(i))
                        {
                            match cmd.kind {
                                15 => s.1 = true,
                                16 => s.0 = !s.0,
                                _ => s.1 = false,
                            }
                        }
                    }
                    29 => {
                        if let Some(v) = usize::try_from(integer(p, 0)?)
                            .ok()
                            .and_then(|i| paths.enabled.get_mut(i))
                        {
                            *v = integer(p, 4)? != 0;
                        }
                    }
                    30 => {
                        let dt = float(p, 0)?;
                        for m in &mut paths.movers {
                            if paths.enabled.get(m.path).copied().unwrap_or(false) {
                                m.elapsed += dt;
                            }
                        }
                    }
                    _ => {}
                }
            }
            let at = (batch.at_seconds * 1000.).round() as u32;
            if let Ok(index) = self.replay.frames.binary_search_by_key(&at, |f| f.at_ms) {
                let state = self.replay.frames[index]
                    .state
                    .as_object_mut()
                    .ok_or("Replay state is not an object")?;
                state.insert("path_enabled".into(), serde_json::json!(paths.enabled));
                state.insert(
                    "movers".into(),
                    serde_json::json!(paths
                        .movers
                        .iter()
                        .map(|m| serde_json::json!({"path":m.path,"elapsed":m.elapsed}))
                        .collect::<Vec<_>>()),
                );
                state.insert(
                    "switches".into(),
                    serde_json::json!(switches
                        .iter()
                        .map(|s| serde_json::json!({"enabled":s.0,"entered":s.1}))
                        .collect::<Vec<_>>()),
                );
                let poses = paths.poses(sol, 0.);
                state.insert("body_poses".into(),serde_json::json!(poses.iter().map(|p|serde_json::json!({"translation":p.translation.to_array(),"rotation":p.rotation.to_array()})).collect::<Vec<_>>()));
            }
        }
        Ok(self)
    }
}
