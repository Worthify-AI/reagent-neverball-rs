// SPDX-License-Identifier: GPL-3.0-or-later
use macroquad::prelude::*;
use reagent_neverball_rs::{Course, Game, Outcome, STEP};
mod visual;
const PAPER: Color = Color::new(0.93, 0.92, 0.88, 1.);
const TEAL: Color = Color::new(0.035, 0.56, 0.66, 1.);
fn conf() -> Conf {
    Conf {
        window_title: "Worthify — Rolling Lab".into(),
        window_width: 800,
        window_height: 600,
        high_dpi: false,
        window_resizable: true,
        ..Default::default()
    }
}
fn paint_text(font: &Font, s: &str, x: f32, y: f32, size: f32, color: Color) {
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
fn button(font: &Font, rect: Rect, label: &str, active: bool) -> bool {
    draw_rectangle(
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        if active {
            TEAL
        } else {
            Color::new(0.18, 0.21, 0.22, 0.92)
        },
    );
    let size = 22.;
    let m = measure_text(label, Some(font), size as u16, 1.);
    paint_text(
        font,
        label,
        rect.x + (rect.w - m.width) / 2.,
        rect.y + rect.h / 2. + 8.,
        size,
        PAPER,
    );
    let (x, y) = mouse_position();
    is_mouse_button_down(MouseButton::Left) && rect.contains(vec2(x, y))
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
    fn worthify_status(phase: i32, coins: i32, seconds: i32);
}
#[macroquad::main(conf)]
async fn main() {
    let font = load_ttf_font_from_bytes(include_bytes!(
        "../assets/neverball/ttf/DejaVuSans-Bold.ttf"
    ))
    .expect("licensed DejaVu Sans Bold font");
    let mut g = Game::new(Course::reference());
    let mut scene = visual::Scene::new();
    let (events, capture) = options();
    #[cfg(target_arch = "wasm32")]
    let _ = capture;
    let tas = !events.is_empty();
    let mut held = std::collections::BTreeSet::new();
    let mut event = 0;
    let mut frame = 0u32;
    let mut accumulator = 0.;
    let mut ready: Option<u32> = None;
    let mut started = false;
    let mut paused = false;
    let mut yaw = 0.;
    let mut wide = false;
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
        if enter && !started {
            ready = Some(if tas { 120 } else { 60 });
        }
        if is_key_pressed(KeyCode::R) || pressed.contains(&114) {
            g.reset();
            started = true;
            ready = None;
            paused = false;
            accumulator = 0.;
        }
        if is_key_pressed(KeyCode::Space)
            || is_key_pressed(KeyCode::Escape)
            || pressed.contains(&32)
        {
            paused = !paused;
        }
        if is_key_pressed(KeyCode::V) || pressed.contains(&118) {
            wide = !wide;
        }
        if is_key_down(KeyCode::Q) {
            yaw -= dt;
        }
        if is_key_down(KeyCode::E) {
            yaw += dt;
        }
        let held_key = |k: KeyCode, code: i32| {
            if tas {
                held.contains(&code)
            } else {
                is_key_down(k)
            }
        };
        let mut input = vec2(
            ((held_key(KeyCode::Right, 1073741903) || (!tas && is_key_down(KeyCode::D))) as u8
                as f32)
                - ((held_key(KeyCode::Left, 1073741904) || (!tas && is_key_down(KeyCode::A))) as u8
                    as f32),
            ((held_key(KeyCode::Up, 1073741906) || (!tas && is_key_down(KeyCode::W))) as u8 as f32)
                - ((held_key(KeyCode::Down, 1073741905) || (!tas && is_key_down(KeyCode::S))) as u8
                    as f32),
        );
        scene.draw(&g, yaw, wide);
        let w = screen_width();
        let h = screen_height();
        let small = w < 550.;
        let scale = (h / 600.).min(w / 800.);
        let timer = format!("{}:{:02}", (g.time as i32) / 60, (g.time as i32) % 60);
        let centis = format!("{:02}", ((g.time.max(0.) * 100.) as i32) % 100);
        let tw = 202. * scale;
        draw_rectangle(
            w * 0.5 - tw * 0.5,
            h - 64. * scale,
            tw,
            64. * scale,
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
        paint_text(
            &font,
            &timer,
            x,
            h - 15. * scale,
            48. * scale,
            Color::new(1., 0.4, 0., 1.),
        );
        paint_text(
            &font,
            &centis,
            w * 0.5 + 42. * scale,
            h - 24. * scale,
            24. * scale,
            ORANGE,
        );
        draw_rectangle(
            w - 146. * scale,
            h - 76. * scale,
            146. * scale,
            76. * scale,
            Color::new(0., 0., 0., 0.45),
        );
        paint_text(
            &font,
            &format!("{}", g.coins()),
            w - 122. * scale,
            h - 48. * scale,
            24. * scale,
            ORANGE,
        );
        paint_text(
            &font,
            "Coins",
            w - 76. * scale,
            h - 48. * scale,
            24. * scale,
            WHITE,
        );
        paint_text(
            &font,
            &format!("{}", g.course.required_coins.saturating_sub(g.coins())),
            w - 130. * scale,
            h - 11. * scale,
            24. * scale,
            ORANGE,
        );
        paint_text(
            &font,
            "Goal",
            w - 76. * scale,
            h - 11. * scale,
            24. * scale,
            WHITE,
        );
        // Touch controls are reserved for small viewports; desktop gameplay
        // retains the original unobstructed presentation.
        if small {
            let size = if small { 46. } else { 42. };
            let pad = vec2(18., h - size * 2. - 26.);
            if button(
                &font,
                Rect::new(pad.x + size + 4., pad.y, size, size),
                "W",
                input.y > 0.,
            ) {
                input.y = 1.;
            }
            if button(
                &font,
                Rect::new(pad.x, pad.y + size + 4., size, size),
                "A",
                input.x < 0.,
            ) {
                input.x = -1.;
            }
            if button(
                &font,
                Rect::new(pad.x + size + 4., pad.y + size + 4., size, size),
                "S",
                input.y < 0.,
            ) {
                input.y = -1.;
            }
            if button(
                &font,
                Rect::new(pad.x + size * 2. + 8., pad.y + size + 4., size, size),
                "D",
                input.x > 0.,
            ) {
                input.x = 1.;
            }
        }
        if !started && ready.is_none() {
            draw_rectangle(
                0.,
                h * 0.30,
                w,
                h * 0.38,
                Color::new(0.075, 0.09, 0.10, 0.9),
            );
            let text = "Neverball · Easy 01";
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
                "Collect 10 coins to unlock the goal.",
                24.,
                h * 0.46,
                if small { 14. } else { 18. },
                PAPER,
            );
            paint_text(&font, "ENTER or tap to begin", 24., h * 0.55, 20., TEAL);
            if is_mouse_button_pressed(MouseButton::Left) {
                ready = Some(60);
            }
        }
        if paused || g.outcome != Outcome::Playing {
            let text = if paused {
                "PAUSED"
            } else {
                match g.outcome {
                    Outcome::Won => "Success!",
                    Outcome::Fell => "TRY AGAIN",
                    Outcome::TimedOut => "TIME UP",
                    _ => "",
                }
            };
            draw_rectangle(0., h * 0.4, w, 100., Color::new(0.075, 0.09, 0.10, 0.9));
            paint_text(&font, text, 24., h * 0.4 + 42., 32., PAPER);
            paint_text(&font, "R / tap RESTART", 24., h * 0.4 + 76., 18., TEAL);
        }
        if small || paused || g.outcome != Outcome::Playing {
            let reset = Rect::new(18., 18., 110., 36.);
            if button(&font, reset, "RESTART", false) && is_mouse_button_pressed(MouseButton::Left)
            {
                g.reset();
                started = true;
                paused = false;
                ready = None;
            }
        }
        if let Some(n) = ready {
            if n == 0 {
                ready = None;
                started = true;
            } else {
                ready = Some(n - 1);
                let msg = if n > 60 { "READY" } else { "SET" };
                paint_text(&font, msg, w * 0.4, h * 0.45, 34., PAPER);
            }
        } else if started && !paused {
            accumulator += dt;
            while accumulator + 1e-7 >= STEP {
                g.step(input, STEP);
                #[cfg(not(target_arch = "wasm32"))]
                {
                    sim_time += STEP;
                }
                accumulator -= STEP;
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = &capture {
            capture_states.push(serde_json::json!({"frame":frame,"sim_time":sim_time,"position":g.position.to_array(),"tilt":g.tilt.to_array(),"input":input.to_array(),"coins":g.coins(),"outcome":format!("{:?}",g.outcome)}));
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
                g.coins() as i32,
                g.time as i32,
            );
        }
        if tas && frame >= 899 {
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
        next_frame().await;
    }
}
