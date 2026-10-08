// SPDX-License-Identifier: GPL-3.0-or-later
use macroquad::prelude::*;
use reagent_neverball_rs::{entities::FullGame, sol::Sol, Outcome, STEP};
mod audio;
mod camera;
mod ui;
mod visual;
use camera::{CameraMode, CameraRig};
use reagent_neverball_rs::replay::{Frame, Playback, Replay};
use reagent_neverball_rs::{
    flow::{self, Flow, Screen},
    settings::SaveData,
};
const PAPER: Color = Color::new(0.93, 0.92, 0.88, 1.);
const TEAL: Color = Color::new(0.035, 0.56, 0.66, 1.);
fn conf() -> Conf {
    #[allow(unused_mut)]
    let mut config = Conf {
        window_title: "Neverball — Rust Reconstruction".into(),
        window_width: 800,
        window_height: 600,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    };
    #[cfg(not(target_arch = "wasm32"))]
    if !std::env::args().any(|a| a == "--tas") {
        if let Ok(save) = SaveData::load(std::path::Path::new(&save_path())) {
            config.window_width = save.settings.width as i32;
            config.window_height = save.settings.height as i32;
            config.fullscreen = save.settings.fullscreen;
            config.sample_count = save.settings.multisample.max(1) as i32;
            config.platform.swap_interval = Some(if save.settings.vsync { 1 } else { 0 });
        }
    }
    config
}
fn bound_mouse(
    settings: &reagent_neverball_rs::settings::Settings,
    action: &str,
    pressed: bool,
) -> bool {
    let button = match settings.bindings.get(action).map(String::as_str) {
        Some("left") => MouseButton::Left,
        Some("right") => MouseButton::Right,
        Some("middle") => MouseButton::Middle,
        _ => return false,
    };
    if pressed {
        is_mouse_button_pressed(button)
    } else {
        is_mouse_button_down(button)
    }
}
fn countdown_text(font: &Font, text: &str, w: f32, h: f32, scale: f32, go: bool) {
    let size = 96. * scale;
    let width = measure_text_guarded(text, Some(font), size as u16, 1.).width;
    let x = (w - width) * 0.5;
    let y = h * 0.55;
    rounded_panel(
        x - 3. * scale,
        y - size * 0.86,
        width + 6. * scale,
        size * 1.12,
        12. * scale,
        Color::new(0., 0., 0., 0.45),
    );
    paint_text(
        font,
        text,
        x + 4. * scale,
        y + 4. * scale,
        size,
        Color::new(0., 0., 0., 0.6),
    );
    gradient_text(
        font,
        text,
        x,
        y,
        size,
        if go {
            vec3(0., 1., 0.)
        } else {
            vec3(1., 0.2, 0.)
        },
        if go {
            vec3(0., 0., 1.)
        } else {
            vec3(1., 1., 0.)
        },
    );
}
fn bound_key(
    settings: &reagent_neverball_rs::settings::Settings,
    action: &str,
    pressed: bool,
) -> bool {
    let wanted = settings.binding_name(action);
    let keys = if pressed {
        get_keys_pressed()
    } else {
        get_keys_down()
    };
    keys.into_iter().any(|key| format!("{key:?}") == wanted)
}
fn screenshot_request() -> Option<String> {
    if !is_key_pressed(KeyCode::F12) {
        return None;
    }
    #[cfg(target_arch = "wasm32")]
    {
        unsafe {
            get_internal_gl().flush();
            worthify_screenshot();
        }
        Some("Screenshot download requested".into())
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let destination = std::path::PathBuf::from(save_path());
        let folder = destination
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join("Screenshots");
        if let Err(e) = std::fs::create_dir_all(&folder) {
            return Some(format!("Screenshot: {e}"));
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let path = folder.join(format!("neverball-{stamp}.png"));
        let frame = get_screen_data();
        let mut pixels =
            ::image::RgbaImage::from_raw(frame.width as u32, frame.height as u32, frame.bytes)
                .expect("framebuffer dimensions");
        ::image::imageops::flip_vertical_in_place(&mut pixels);
        Some(match pixels.save(&path) {
            Ok(()) => format!("Screenshot saved: {}", path.display()),
            Err(e) => format!("Screenshot: {e}"),
        })
    }
}
fn prepare_text(s: &str, size: u16) {
    thread_local! { static GLYPHS: std::cell::RefCell<std::collections::HashSet<(char,u16)>> = std::cell::RefCell::new(std::collections::HashSet::new()); }
    let new_glyphs = GLYPHS.with(|seen| {
        let mut seen = seen.borrow_mut();
        let mut changed = false;
        for c in s.chars() {
            changed |= seen.insert((c, size));
        }
        changed
    });
    if new_glyphs {
        // Atlas growth replaces its GPU texture. Finish queued uses and clear
        // miniquad bindings before Macroquad deletes the previous atlas.
        unsafe {
            let mut gl = get_internal_gl();
            gl.flush();
            gl.quad_context.commit_frame();
        }
    }
}
fn measure_text_guarded(text: &str, font: Option<&Font>, size: u16, scale: f32) -> TextDimensions {
    prepare_text(text, size);
    measure_text(text, font, size, scale)
}
fn paint_text(font: &Font, s: &str, x: f32, y: f32, size: f32, color: Color) {
    prepare_text(s, size as u16);
    // Macroquad otherwise caches/draws glyphs one at a time. Pre-cache the
    // entire run so atlas growth cannot invalidate earlier glyphs in this draw.
    let _ = measure_text(s, Some(font), size as u16, 1.);
    draw_text_ex(
        s,
        x,
        y,
        TextParams {
            font: Some(font),
            font_size: size as u16,
            color,
            ..Default::default()
        },
    );
}
fn rounded_panel(x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    let mut edge = Vec::new();
    for (cx, cy, start) in [
        (x + w - r, y + r, -90f32),
        (x + w - r, y + h - r, 0.),
        (x + r, y + h - r, 90.),
        (x + r, y + r, 180.),
    ] {
        for i in 0..=8 {
            let a = (start + i as f32 * 90. / 8.).to_radians();
            edge.push(vec2(cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    let center = vec2(x + w * 0.5, y + h * 0.5);
    for i in 0..edge.len() {
        draw_triangle(center, edge[i], edge[(i + 1) % edge.len()], color);
    }
}
fn gradient_text(font: &Font, text: &str, x: f32, y: f32, size: f32, top: Vec3, bottom: Vec3) {
    // Keep font atlas rendering on Macroquad's standard text material. A font
    // atlas can grow when result-screen glyphs are added during this frame.
    let first = (y - size).floor() as i32;
    let last = (y + size * 0.25).ceil() as i32;
    for row in first..last {
        let t = ((row as f32 - (y - size * 0.76)) / (size * 0.76)).clamp(0., 1.);
        let rgb = top.lerp(bottom, t);
        unsafe {
            get_internal_gl()
                .quad_gl
                .scissor(Some((0, row, screen_width() as i32, 1)));
        }
        paint_text(font, text, x, y, size, Color::new(rgb.x, rgb.y, rgb.z, 1.));
    }
    unsafe {
        get_internal_gl().quad_gl.scissor(None);
    }
}
#[derive(Clone, Copy)]
struct Event {
    frame: u32,
    key: i32,
    down: bool,
}
#[cfg(not(target_arch = "wasm32"))]
fn options() -> (Vec<Event>, Option<String>) {
    let args: Vec<_> = std::env::args().collect();
    let mut events = Vec::new();
    let mut capture = None;
    for i in 1..args.len() {
        if args[i] == "--tas" {
            let s = std::fs::read_to_string(&args[i + 1]).expect("TAS file");
            for l in s.lines() {
                let v: Vec<_> = l.split_whitespace().collect();
                if v.len() == 3 {
                    events.push(Event {
                        frame: v[0].parse().unwrap(),
                        key: v[1].parse().unwrap(),
                        down: v[2] == "1",
                    });
                }
            }
        }
        if args[i] == "--capture" {
            let path = args[i + 1].clone();
            std::fs::create_dir_all(&path).unwrap();
            capture = Some(path);
        }
    }
    (events, capture)
}
#[cfg(target_arch = "wasm32")]
fn options() -> (Vec<Event>, Option<String>) {
    (Vec::new(), None)
}
#[cfg(target_arch = "wasm32")]
extern "C" {
    fn worthify_replay_load(ptr: *mut u8, capacity: usize) -> usize;
    fn worthify_screenshot();
    fn worthify_axis_active() -> i32;
    fn worthify_axis_x() -> f32;
    fn worthify_axis_y() -> f32;
    fn worthify_status(phase: i32, coins: i32, seconds: i32, target: i32);
    fn worthify_save(ptr: *const u8, len: usize) -> i32;
    fn worthify_load(ptr: *mut u8, capacity: usize) -> usize;
}
fn data_options() -> (String, String) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<_> = std::env::args().collect();
        let value = |name: &str, default: &str| {
            args.windows(2)
                .find(|a| a[0] == name)
                .map(|a| a[1].clone())
                .unwrap_or_else(|| default.into())
        };
        (
            value("--data", "data"),
            value("--level", "map-easy/easy.sol"),
        )
    }
    #[cfg(target_arch = "wasm32")]
    {
        ("data".into(), "map-easy/easy.sol".into())
    }
}
async fn launch_level(
    root: &str,
    launch: &flow::Launch,
) -> Result<(FullGame, visual::Scene), String> {
    let bytes = load_file(&format!("{root}/{}", launch.level_id))
        .await
        .map_err(|e| e.to_string())?;
    let sol = Sol::from_bytes(&bytes).map_err(|e| e.to_string())?;
    let scene = visual::Scene::load(&sol, root).await?;
    let mut game = FullGame::new(sol);
    if launch.goal_open {
        game.required_coins = 0;
    }
    Ok((game, scene))
}
fn replay_frame(g: &FullGame, camera: &CameraRig) -> Frame {
    let matrix = Mat3::from_quat(g.ball.orientation);
    Frame {
        at_ms: (g.elapsed * 1000.).round() as u32,
        position: g.ball.position.to_array(),
        orientation: [
            matrix.x_axis.to_array(),
            matrix.y_axis.to_array(),
            matrix.z_axis.to_array(),
        ],
        radius: g.ball.radius,
        camera_eye: camera.eye.to_array(),
        camera_target: camera.target.to_array(),
        tilt: g.tilt.to_array(),
        coins: g.coins,
        remaining_seconds: g.time,
        state: serde_json::json!({"collected":g.collected,"body_poses":g.body_poses.iter().map(|p|serde_json::json!({"translation":p.translation.to_array(),"rotation":p.rotation.to_array()})).collect::<Vec<_>>(),"camera_right":camera.right.to_array(),"camera_back":camera.back.to_array()}),
    }
}
fn apply_replay(g: &mut FullGame, camera: &mut CameraRig, f: &Frame) {
    g.ball.position = Vec3::from_array(f.position);
    g.ball.radius = f.radius;
    g.ball.orientation = Quat::from_mat3(&Mat3::from_cols_array_2d(&f.orientation));
    g.tilt = Vec2::from_array(f.tilt);
    g.coins = f.coins;
    g.time = f.remaining_seconds;
    camera.eye = Vec3::from_array(f.camera_eye);
    camera.target = Vec3::from_array(f.camera_target);
    camera.right = Vec3::Y
        .cross(camera.eye - camera.target)
        .normalize_or_zero();
    camera.back = camera.right.cross(Vec3::Y).normalize_or_zero();
    if let Ok(collected) = serde_json::from_value::<Vec<bool>>(f.state["collected"].clone()) {
        if collected.len() == g.collected.len() {
            g.collected = collected;
        }
    }
    if let Some(poses) = f.state["body_poses"].as_array() {
        for (pose, value) in g.body_poses.iter_mut().zip(poses) {
            if let (Ok(p), Ok(q)) = (
                serde_json::from_value::<[f32; 3]>(value["translation"].clone()),
                serde_json::from_value::<[f32; 4]>(value["rotation"].clone()),
            ) {
                pose.translation = Vec3::from_array(p);
                pose.rotation = Quat::from_array(q);
            }
        }
    }
}
#[cfg(not(target_arch = "wasm32"))]
fn save_path() -> String {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<_> = std::env::args().collect();
        args.windows(2)
            .find(|a| a[0] == "--save")
            .map(|a| a[1].clone())
            .unwrap_or_else(|| "neverball-rust-save.json".into())
    }
    #[cfg(target_arch = "wasm32")]
    {
        String::new()
    }
}
fn save_progress(save: &SaveData) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        match save.save(std::path::Path::new(&save_path())) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("Could not save progress: {e}");
                false
            }
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        save.to_json()
            .ok()
            .is_some_and(|json| unsafe { worthify_save(json.as_ptr(), json.len()) != 0 })
    }
}
#[macroquad::main(conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!(
        "../assets/neverball/ttf/DejaVuSans-Bold.ttf"
    ))
    .expect("licensed DejaVu Sans Bold font");
    let (data_root, level_path) = data_options();
    let bytes = load_file(&format!("{data_root}/{level_path}"))
        .await
        .expect("level data missing: supply --data with the packaged runtime data directory");
    let mut g = FullGame::new(Sol::from_bytes(&bytes).expect("compiled level data"));
    let mut scene = visual::Scene::load(&g.sol, &data_root)
        .await
        .expect("level visual assets");
    let (events, capture) = options();
    #[cfg(target_arch = "wasm32")]
    let _ = capture;
    let tas = !events.is_empty();
    #[cfg(not(target_arch = "wasm32"))]
    let capture_frames = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|a| a[0] == "--frames")
        .and_then(|a| a[1].parse::<u32>().ok())
        .unwrap_or(900);
    #[cfg(target_arch = "wasm32")]
    let capture_frames = 900u32;
    let mut ui = ui::Ui::load(&data_root).await.expect("level-set catalog");
    #[cfg(not(target_arch = "wasm32"))]
    let mut save =
        SaveData::load(std::path::Path::new(&save_path())).unwrap_or_else(|_| SaveData::new());
    #[cfg(target_arch = "wasm32")]
    let mut save = {
        let size = unsafe { worthify_load(std::ptr::null_mut(), 0) };
        let mut bytes = vec![0u8; size];
        unsafe {
            worthify_load(bytes.as_mut_ptr(), size);
        };
        String::from_utf8(bytes)
            .ok()
            .and_then(|s| SaveData::from_json(&s).ok())
            .unwrap_or_default()
    };
    for (set, levels) in ui.catalog.sets.iter().zip(&ui.levels) {
        let progress =
            save.progress
                .sets
                .entry(set.id.clone())
                .or_insert_with(|| flow::SetProgress {
                    challenge_records: flow::Records::reference_set(set),
                    ..Default::default()
                });
        for level in levels {
            progress
                .records
                .entry(level.path.clone())
                .or_insert_with(|| flow::Records::reference_level(level));
        }
    }
    let mut settings = save.settings.clone();
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(mode) = std::env::args()
        .collect::<Vec<_>>()
        .windows(2)
        .find(|a| a[0] == "--camera")
        .and_then(|a| a[1].parse::<u8>().ok())
        .filter(|m| (1..=3).contains(m))
    {
        settings.camera = mode;
    }

    let mut flow = Flow::new(ui.specs(), save.progress.clone());
    flow.lock_completed_goals = settings.lock_goals;
    if tas {
        let selected = ui
            .catalog
            .sets
            .iter()
            .enumerate()
            .find_map(|(si, set)| {
                set.levels
                    .iter()
                    .position(|p| p == &level_path)
                    .map(|li| (si, li))
            })
            .unwrap_or((0, 0));
        flow.progress
            .sets
            .entry(ui.catalog.sets[selected.0].id.clone())
            .or_default()
            .unlocked
            .insert(level_path.clone());
        flow.start(selected.0, selected.1, flow::Mode::Normal)
            .unwrap();
    }
    let mut sounds = audio::SoundBank::load(&data_root)
        .await
        .expect("licensed sounds");
    let song = if tas {
        g.sol
            .metadata
            .get("song")
            .map(String::as_str)
            .unwrap_or("bgm/title.ogg")
    } else {
        "bgm/title.ogg"
    };
    let _ = sounds.set_song(song).await;
    if !tas {
        let bytes = load_file(&format!("{data_root}/map-medium/title.sol"))
            .await
            .expect("title course");
        g = FullGame::new(Sol::from_bytes(&bytes).unwrap());
        scene = visual::Scene::load(&g.sol, &data_root)
            .await
            .expect("title art");
    }
    let mut recording = Replay::new("", &level_path, &settings.player);
    let mut result_age = 0f32;
    let mut result_input = Vec2::ZERO;
    let mut result_rotation = 0f32;
    let mut replay_clock = 0f32;
    let mut playback: Option<Playback> = None;
    let mut playback_name = String::from("Last");
    #[cfg(not(target_arch = "wasm32"))]
    {
        let args: Vec<_> = std::env::args().collect();
        if let Some(file) = args
            .windows(2)
            .find(|a| a[0] == "--replay-file")
            .map(|a| &a[1])
        {
            match std::fs::read(file) {
                Ok(bytes) => match import_replay(&data_root, &bytes).await {
                    Ok((replay, game, view)) => {
                        g = game;
                        scene = view;
                        save.replays.set_last(replay.clone());
                        playback = Some(Playback::new(replay));
                    }
                    Err(e) => ui.status = e,
                },
                Err(e) => ui.status = e.to_string(),
            }
            flow.screen = Screen::Replays;
        }
    }
    let mut pointer_tilt = Vec2::ZERO;
    let mut keyboard_was_active = false;
    let mut applied_display = (settings.width, settings.height, settings.fullscreen);
    let mut held = std::collections::BTreeSet::new();
    let mut event = 0;
    let mut frame = 0u32;
    let mut accumulator = 0.;
    let mut ready: Option<u32> = None;
    let mut started = false;
    let mut paused = false;
    let mut camera = CameraRig::new(g.ball.position);
    #[cfg(not(target_arch = "wasm32"))]
    let mut capture_states: Vec<serde_json::Value> = Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    let mut sim_time = 0.;
    loop {
        let dt = if tas {
            1. / 60.
        } else {
            get_frame_time().min(0.1)
        };
        if scene.ball_file != settings.ball_file {
            if let Err(e) = scene.set_ball(&data_root, &settings.ball_file).await {
                ui.status = e;
                settings.ball_file = scene.ball_file.clone();
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let size = unsafe { worthify_replay_load(std::ptr::null_mut(), 0) };
            if size > 0 {
                let mut bytes = vec![0u8; size];
                unsafe {
                    worthify_replay_load(bytes.as_mut_ptr(), size);
                }
                match import_replay(&data_root, &bytes).await {
                    Ok((replay, game, view)) => {
                        save.replays.set_last(replay.clone());
                        g = game;
                        scene = view;
                        playback = Some(Playback::new(replay));
                    }
                    Err(e) => {
                        ui.status = e;
                        flow.screen = Screen::Replays;
                    }
                }
            }
        }
        let display = (settings.width, settings.height, settings.fullscreen);
        if !tas && display != applied_display {
            if settings.fullscreen != applied_display.2 {
                set_fullscreen(settings.fullscreen);
            }
            request_new_screen_size(settings.width as f32, settings.height as f32);
            applied_display = display;
        }
        scene.animation_time += dt;
        if g.goal_open() {
            scene.goal_height = if g.elapsed == 0. {
                1.
            } else {
                (scene.goal_height + dt).min(1.)
            };
        } else {
            scene.goal_height = 0.;
        }
        scene.background_enabled = settings.background;
        scene.fov = settings.view_fov as f32;
        scene.set_filtering(settings.mipmap);
        scene.textures_enabled = settings.textures;
        scene.shadow_enabled = settings.shadow;
        scene.reflection_enabled = settings.reflection;
        camera.mode = match settings.camera {
            1 => CameraMode::Chase,
            3 => CameraMode::Manual,
            _ => CameraMode::Lazy,
        };
        if !started {
            camera.preview(
                g.ball.position,
                &g.sol.views,
                ready.unwrap_or(120) as f32 / 120.,
            );
        }
        sounds.volumes(settings.sound_gain(), settings.music_gain());
        if is_mouse_button_pressed(MouseButton::Left) || !get_keys_pressed().is_empty() {
            sounds.start_music();
        }
        if let Some(replay) = &mut playback {
            #[cfg(target_arch = "wasm32")]
            unsafe {
                worthify_status(7, g.coins, g.time as i32, g.required_coins);
            }

            if is_key_pressed(KeyCode::Space) {
                replay.paused = !replay.paused;
            }
            if is_key_pressed(KeyCode::R) {
                replay.restart();
            }
            if is_key_pressed(KeyCode::Right) {
                replay.seek(replay.at_ms as u32 + 5000);
            }
            if is_key_pressed(KeyCode::Left) {
                replay.seek((replay.at_ms as u32).saturating_sub(5000));
            }
            if let Some(f) = replay.advance(dt as f64).cloned() {
                apply_replay(&mut g, &mut camera, &f);
            }
            scene.draw(&g, &camera);
            if ui.ball_preview_requested() {
                scene.ball_preview(get_time() as f32);
            }
            paint_text(
                &font,
                if replay.ended {
                    "Replay complete · R Repeat · Esc Back"
                } else {
                    "Replay · Space Pause · ← / → Seek · Esc Back"
                },
                20.,
                35.,
                20.,
                WHITE,
            );
            let mut leave = false;
            if replay.ended {
                match ui.draw_replay_end(&font) {
                    ui::ReplayEndAction::Repeat => replay.restart(),
                    ui::ReplayEndAction::Delete => {
                        if playback_name == "Last" {
                            save.replays.last = None;
                        } else {
                            save.replays.delete(&playback_name);
                        }
                        save_progress(&save);
                        leave = true;
                    }
                    ui::ReplayEndAction::Keep => leave = true,
                    ui::ReplayEndAction::None => {}
                }
            }
            if leave || is_key_pressed(KeyCode::Escape) {
                playback = None;
                flow.screen = Screen::Replays;
            }
            if let Some(message) = screenshot_request() {
                ui.status = message;
            }
            next_frame().await;
            continue;
        }
        if matches!(
            flow.screen,
            Screen::Title
                | Screen::Sets
                | Screen::Levels
                | Screen::Help
                | Screen::Options
                | Screen::Replays
                | Screen::SetComplete
                | Screen::GameOver
        ) {
            if !recording.frames.is_empty() && flow.screen == Screen::Levels {
                save.replays.set_last(recording.clone());
                recording.frames.clear();
                save_progress(&save);
            }
            #[cfg(target_arch = "wasm32")]
            unsafe {
                worthify_status(6, g.coins, g.time as i32, g.required_coins);
            }
            scene.draw(&g, &camera);
            if ui.ball_preview_requested() {
                scene.ball_preview(get_time() as f32);
            }
            ui.prepare_preview(&data_root, flow.screen).await;
            match ui.draw(&font, &mut flow, &mut settings, &save.replays) {
                ui::Action::Launch(launch) => match launch_level(&data_root, &launch).await {
                    Ok((game, view)) => {
                        g = game;
                        scene = view;
                        camera.reset(g.ball.position);
                        ready = None;
                        started = false;
                        paused = false;
                        accumulator = 0.;
                        let _ = sounds
                            .set_song(g.sol.metadata.get("song").map(String::as_str).unwrap_or(""))
                            .await;
                        sounds.start_music();
                    }
                    Err(e) => {
                        ui.status = e;
                        flow.screen = Screen::Levels;
                    }
                },
                ui::Action::PlayReplay(name) => {
                    playback_name = name.clone();
                    let chosen = if name == "Last" {
                        save.replays.last.clone()
                    } else {
                        save.replays.saved.get(&name).cloned()
                    };
                    if let Some(replay) = chosen {
                        let launch = flow::Launch {
                            set_id: replay.set_id.clone(),
                            level_id: replay.level_id.clone(),
                            mode: flow::Mode::Normal,
                            goal_open: false,
                        };
                        match launch_level(&data_root, &launch).await {
                            Ok((game, view)) => {
                                g = game;
                                scene = view;
                                playback = Some(Playback::new(replay));
                            }
                            Err(e) => ui.status = e,
                        }
                    }
                }
                ui::Action::SaveReplay(name) => {
                    ui.status = match save.replays.save_last(&name, false) {
                        Ok(()) => "Replay kept".into(),
                        Err(e) => e,
                    };
                    if !save_progress(&save) {
                        ui.status = "Replay is in memory; storage is full or unavailable.".into();
                    }
                }
                ui::Action::DeleteReplay(name) => {
                    save.replays.delete(&name);
                    save_progress(&save);
                }
                ui::Action::Exit => {
                    sounds.stop_music();
                    break;
                }
                _ => {}
            }
            save.settings = settings.clone();
            save.progress = flow.progress.clone();
            if is_mouse_button_pressed(MouseButton::Left) || !get_keys_pressed().is_empty() {
                save_progress(&save);
                sounds.cue("menu", 1.);
            }
            if let Some(message) = screenshot_request() {
                ui.status = message;
            }
            next_frame().await;
            continue;
        }
        let mut pressed = std::collections::BTreeSet::new();
        let mut enter = is_key_pressed(KeyCode::Enter);
        while event < events.len() && events[event].frame <= frame {
            let e = events[event];
            if e.down {
                held.insert(e.key);
                pressed.insert(e.key);
            } else {
                held.remove(&e.key);
            }
            if e.key == 13 && e.down {
                enter = true;
            }
            event += 1;
        }
        if enter && !started && flow.screen == Screen::Intro {
            ready = Some(120);
            sounds.cue("ready", 1.);
        }
        if (if tas {
            pressed.contains(&114)
        } else {
            bound_key(&settings, "key_restart", true)
        }) && flow.retry().is_ok()
        {
            g.reset();
            camera.reset(g.ball.position);
            started = false;
            ready = Some(120);
            paused = false;
            accumulator = 0.;
        }
        if (is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Escape)
            || pressed.contains(&32)
            || pressed.contains(&27))
            && started
            && g.outcome == Outcome::Playing
        {
            paused = !paused;
            if paused {
                flow.pause();
            } else {
                flow.resume();
            }
            sounds.mute_music(paused);
        }
        for (key, code, mode) in [
            ("key_camera_1", 49, CameraMode::Chase),
            ("key_camera_2", 50, CameraMode::Lazy),
            ("key_camera_3", 51, CameraMode::Manual),
        ] {
            if if tas {
                pressed.contains(&code)
            } else {
                bound_key(&settings, key, true)
            } {
                camera.mode = mode;
                settings.camera = code as u8 - 48;
            }
        }
        if (if tas {
            pressed.contains(&101)
        } else {
            bound_key(&settings, "key_camera_toggle", true)
        }) || (!tas && bound_mouse(&settings, "mouse_camera_toggle", true))
        {
            camera.toggle();
            settings.camera = if camera.mode == CameraMode::Manual {
                3
            } else {
                1
            };
        }
        let held_key = |action: &str, code: i32| {
            if tas {
                held.contains(&code)
            } else {
                bound_key(&settings, action, false)
            }
        };
        let mut input = vec2(
            held_key("key_right", 1073741903) as u8 as f32
                - held_key("key_left", 1073741904) as u8 as f32,
            held_key("key_forward", 1073741906) as u8 as f32
                - held_key("key_backward", 1073741905) as u8 as f32,
        );
        if !tas {
            let keyboard_active = input.length_squared() > 0.;
            if keyboard_active || keyboard_was_active {
                pointer_tilt = input;
                g.response_seconds = settings.joystick_response as f32 / 1000.;
            }
            let delta = mouse_delta_position() * vec2(screen_width(), screen_height()) * 0.5;
            if started && !paused && delta.length_squared() > 0. {
                pointer_tilt += vec2(
                    -delta.x,
                    if settings.mouse_invert {
                        -delta.y
                    } else {
                        delta.y
                    },
                ) * (2. / settings.mouse_sense.max(1) as f32);
                pointer_tilt = pointer_tilt.clamp(Vec2::splat(-1.), Vec2::ONE);
                g.response_seconds = settings.mouse_response as f32 / 1000.;
            }
            input = pointer_tilt;
            keyboard_was_active = keyboard_active;
            #[cfg(target_arch = "wasm32")]
            unsafe {
                let axis_kind = worthify_axis_active();
                if !keyboard_active && (axis_kind == 1 || (axis_kind == 2 && settings.joystick)) {
                    pointer_tilt = Vec2::ZERO;
                    input = vec2(worthify_axis_x(), worthify_axis_y())
                        .clamp(Vec2::splat(-1.), Vec2::ONE);
                    g.response_seconds = settings.joystick_response as f32 / 1000.;
                }
            }
        }
        let rotate_right = held_key("key_camera_r", 100)
            || (!tas && started && bound_mouse(&settings, "mouse_camera_r", false));
        let rotate_left = held_key("key_camera_l", 115)
            || (!tas && started && bound_mouse(&settings, "mouse_camera_l", false));
        let fast =
            held_key("key_rotate_fast", 1073742049) || held_key("key_rotate_fast", 1073742053);
        let rotation = (rotate_right as u8 as f32 - rotate_left as u8 as f32)
            * if fast {
                settings.rotate_fast as f32 / 100.
            } else {
                settings.rotate_slow as f32 / 100.
            };
        scene.draw(&g, &camera);
        let w = screen_width();
        let h = screen_height();
        let small = w < 550.;
        let scale = (h / 600.).min(w / 800.);
        let timer = format!("{}:{:02}", (g.time as i32) / 60, (g.time as i32) % 60);
        let centis = format!("{:02}", ((g.time.max(0.) * 100.) as i32) % 100);
        let tw = 202. * scale;
        rounded_panel(
            w * 0.5 - tw * 0.5,
            h - 64. * scale,
            tw,
            76. * scale,
            12. * scale,
            Color::new(0., 0., 0., 0.45),
        );
        let x = w * 0.5 - 71. * scale;
        paint_text(
            &font,
            &timer,
            x + 2. * scale,
            h - 13. * scale,
            48. * scale,
            BLACK,
        );
        gradient_text(
            &font,
            &timer,
            x,
            h - 15. * scale,
            48. * scale,
            vec3(1., 0.2, 0.),
            vec3(1., 1., 0.),
        );
        paint_text(
            &font,
            &centis,
            w * 0.5 + 42. * scale,
            h - 24. * scale,
            24. * scale,
            ORANGE,
        );
        rounded_panel(
            w - 146. * scale,
            h - 76. * scale,
            160. * scale,
            88. * scale,
            12. * scale,
            Color::new(0., 0., 0., 0.45),
        );
        paint_text(
            &font,
            &format!("{}", g.coins),
            w - 122. * scale,
            h - 48. * scale,
            24. * scale,
            ORANGE,
        );
        paint_text(
            &font,
            &ui::translate("Coins"),
            w - 76. * scale,
            h - 48. * scale,
            24. * scale,
            WHITE,
        );
        paint_text(
            &font,
            &format!("{}", (g.required_coins - g.coins).max(0)),
            w - 130. * scale,
            h - 11. * scale,
            24. * scale,
            ORANGE,
        );
        paint_text(
            &font,
            &ui::translate("Goal"),
            w - 76. * scale,
            h - 11. * scale,
            24. * scale,
            WHITE,
        );
        if let Some(session) = flow
            .session
            .as_ref()
            .filter(|s| s.mode == flow::Mode::Challenge)
        {
            draw_rectangle(
                0.,
                h - 64. * scale,
                160. * scale,
                64. * scale,
                Color::new(0., 0., 0., 0.45),
            );
            paint_text(
                &font,
                &format!(
                    "{} {}",
                    session.balls.saturating_sub(1),
                    ui::translate("Balls")
                ),
                18. * scale,
                h - 18. * scale,
                28. * scale,
                ORANGE,
            );
        }
        if !started && ready.is_none() {
            draw_rectangle(
                0.,
                h * 0.30,
                w,
                h * 0.38,
                Color::new(0.075, 0.09, 0.10, 0.9),
            );
            let title = flow
                .session
                .as_ref()
                .map(|session| {
                    format!(
                        "{} · Level {:02}",
                        ui.catalog.sets[session.set].title,
                        session.level + 1
                    )
                })
                .unwrap_or_else(|| level_path.clone());
            let text = &title;
            paint_text(
                &font,
                text,
                24.,
                h * 0.38,
                if small { 25. } else { 38. },
                PAPER,
            );
            paint_text(
                &font,
                &format!("Collect {} coins to unlock the goal.", g.required_coins),
                24.,
                h * 0.46,
                if small { 14. } else { 18. },
                PAPER,
            );
            paint_text(&font, "ENTER or tap to begin", 24., h * 0.55, 20., TEAL);
            if is_mouse_button_pressed(MouseButton::Left) {
                ready = Some(120);
                sounds.cue("ready", 1.);
            }
        }
        if paused || g.outcome != Outcome::Playing {
            match ui.draw(&font, &mut flow, &mut settings, &save.replays) {
                ui::Action::Resume => {
                    paused = false;
                    flow.resume();
                    sounds.mute_music(false);
                }
                ui::Action::Retry => {
                    if let Ok(launch) = flow.retry() {
                        if let Ok((game, view)) = launch_level(&data_root, &launch).await {
                            g = game;
                            scene = view;
                            camera.reset(g.ball.position);
                            started = false;
                            paused = false;
                            ready = None;
                            accumulator = 0.;
                        }
                    }
                }
                ui::Action::Next => {
                    if let Ok(Some(launch)) = flow.next() {
                        if let Ok((game, view)) = launch_level(&data_root, &launch).await {
                            g = game;
                            scene = view;
                            camera.reset(g.ball.position);
                            started = false;
                            paused = false;
                            ready = None;
                            accumulator = 0.;
                            let _ = sounds
                                .set_song(
                                    g.sol.metadata.get("song").map(String::as_str).unwrap_or(""),
                                )
                                .await;
                            sounds.start_music();
                        }
                    }
                }
                ui::Action::SaveReplay(name) => {
                    ui.status = match save.replays.save_last(&name, false) {
                        Ok(()) => "Replay kept".into(),
                        Err(e) => e,
                    };
                    if !save_progress(&save) {
                        ui.status = "Replay is in memory; storage is full or unavailable.".into();
                    }
                }
                _ => {}
            }
        }
        if !started || ready.is_some() {
            let name = ui::translate(match camera.mode {
                CameraMode::Chase => "Chase Camera",
                CameraMode::Lazy => "Lazy Camera",
                CameraMode::Manual => "Manual Camera",
            });
            let size = 24. * scale;
            let width = measure_text(&name, Some(&font), size as u16, 1.).width;
            rounded_panel(
                w - width - 12. * scale,
                -12. * scale,
                width + 24. * scale,
                50. * scale,
                12. * scale,
                Color::new(0., 0., 0., 0.45),
            );
            paint_text(
                &font,
                &name,
                w - width - 6. * scale,
                27. * scale,
                size,
                WHITE,
            );
        }
        if let Some(n) = ready {
            if n == 0 {
                ready = None;
                started = true;
                camera.reset(g.ball.position);
                flow.begin_play();
                sounds.cue("go", 1.);
                let session = flow.session.as_ref().unwrap();
                result_age = 0.;
                replay_clock = 0.;
                recording = Replay::new(
                    &ui.catalog.sets[session.set].id,
                    &ui.catalog.sets[session.set].levels[session.level],
                    &settings.player,
                );
            } else {
                if n == 60 {
                    sounds.cue("set", 1.);
                }
                ready = Some(n - 1);
                let msg = ui::translate(if n > 60 { "Ready?" } else { "Set?" });
                countdown_text(&font, &msg, w, h, scale, false);
            }
        } else if started && !paused {
            if g.elapsed < 0.5 && flow.screen == Screen::Playing {
                countdown_text(&font, &ui::translate("GO!"), w, h, scale, true);
            }
            accumulator += dt;
            while accumulator + 1e-7 >= STEP {
                if g.outcome == Outcome::Playing {
                    result_age = 0.;
                    result_input = input;
                    result_rotation = rotation;
                    let previous_position = g.ball.position;
                    g.step(input, camera.right, camera.back, STEP);
                    let teleported = g.events.iter().any(|event| {
                        matches!(event, reagent_neverball_rs::entities::Event::Teleported(_))
                    });
                    camera.step_jump(
                        g.ball.position,
                        g.ball.velocity,
                        rotation,
                        STEP,
                        g.jump.as_ref().map(|jump| jump.elapsed),
                        if teleported {
                            g.ball.position - previous_position
                        } else {
                            Vec3::ZERO
                        },
                    );
                } else {
                    let duration = match g.outcome {
                        Outcome::Won => 1.,
                        Outcome::Fell => 2.,
                        _ => 0.,
                    };
                    if result_age < duration {
                        g.step_result(result_input, camera.right, camera.back, STEP);
                        camera.step(g.ball.position, g.ball.velocity, result_rotation, STEP);
                    }
                    result_age += STEP;
                }
                sounds.events(&g);
                g.events.clear();
                replay_clock += STEP;
                let mut replay_sample = replay_frame(&g, &camera);
                replay_sample.at_ms = (replay_clock * 1000.) as u32;
                if g.outcome == Outcome::Playing || result_age <= 2. {
                    let _ = recording.push(replay_sample);
                }
                if g.outcome != Outcome::Playing && result_age <= 2. {
                    save.replays.set_last(recording.clone());
                }
                if result_age > 2. && result_age - STEP <= 2. {
                    save_progress(&save);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    sim_time += STEP;
                }
                accumulator -= STEP;
            }
        }
        if g.outcome != Outcome::Playing && flow.screen == Screen::Playing {
            let outcome = match g.outcome {
                Outcome::Won => flow::Outcome::Success,
                Outcome::Fell => flow::Outcome::FallOut,
                _ => flow::Outcome::TimeOut,
            };
            let _ = flow.finish(
                outcome,
                g.coins.max(0) as u32,
                if g.time_limit > 0. {
                    10 * ((g.time_limit * 100.) as u32).saturating_sub((g.time * 100.) as u32)
                } else {
                    10 * (g.elapsed * 100.) as u32
                },
                &settings.player,
            );
            recording.outcome = Some(outcome);
            save.replays.set_last(recording.clone());
            save.progress = flow.progress.clone();
            save_progress(&save);
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &capture {
            capture_states.push(serde_json::json!({"frame":frame,"sim_time":sim_time,"position":g.ball.position.to_array(),"tilt":g.tilt.to_array(),"input":input.to_array(),"camera_eye":camera.eye.to_array(),"camera_target":camera.target.to_array(),"camera_right":camera.right.to_array(),"camera_back":camera.back.to_array(),"orientation":g.ball.orientation.to_array(),"velocity":g.ball.velocity.to_array(),"camera_mode":format!("{:?}",camera.mode),"camera_rotation":rotation,"coins":g.coins,"outcome":format!("{:?}",g.outcome)}));
            if frame.is_multiple_of(2) {
                get_screen_data().export_png(&format!("{path}/frame-{frame:05}.png"));
            }
        }
        #[cfg(target_arch = "wasm32")]
        unsafe {
            worthify_status(
                if paused {
                    2
                } else if !started {
                    0
                } else {
                    match g.outcome {
                        Outcome::Playing => 1,
                        Outcome::Won => 3,
                        Outcome::Fell => 4,
                        Outcome::TimedOut => 5,
                    }
                },
                g.coins as i32,
                g.time as i32,
                g.required_coins,
            );
        }
        if tas && frame + 1 >= capture_frames {
            #[cfg(not(target_arch = "wasm32"))]
            if let Some(path) = &capture {
                std::fs::write(
                    format!("{path}/states.json"),
                    serde_json::to_vec(&capture_states).unwrap(),
                )
                .unwrap();
            }
            break;
        }
        frame += 1;
        if let Some(message) = screenshot_request() {
            ui.status = message;
        }
        next_frame().await;
    }
}

async fn import_replay(
    root: &str,
    bytes: &[u8],
) -> Result<(Replay, FullGame, visual::Scene), String> {
    let replay = if bytes.starts_with(&[0xaf, b'N', b'B', b'R']) {
        let original = reagent_neverball_rs::replay::OriginalReplay::from_nbr(bytes)
            .map_err(|e| e.to_string())?;
        if !original.unhandled_kinds.is_empty() {
            return Err(format!(
                "Unsupported replay commands: {:?}",
                original.unhandled_kinds
            ));
        }
        let bytes = load_file(&format!("{root}/{}", original.header.level))
            .await
            .map_err(|e| e.to_string())?;
        let sol = Sol::from_bytes(&bytes).map_err(|e| e.to_string())?;
        original.with_sol(&sol).map_err(|e| e.to_string())?.replay
    } else {
        serde_json::from_slice::<Replay>(bytes).map_err(|e| e.to_string())?
    };
    let launch = flow::Launch {
        set_id: replay.set_id.clone(),
        level_id: replay.level_id.clone(),
        mode: flow::Mode::Normal,
        goal_open: false,
    };
    let (game, scene) = launch_level(root, &launch).await?;
    Ok((replay, game, scene))
}
