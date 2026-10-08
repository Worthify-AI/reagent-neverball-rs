// SPDX-License-Identifier: GPL-3.0-or-later
// Raw keyboard holdout for the retained offscreen mover capture. Event-clock
// origin is aligned at the first input transition; no position, velocity,
// camera, tilt or mover state from the replay is applied to the reconstruction.
use macroquad::prelude::*;
use reagent_neverball_rs::camera::CameraRig;
use reagent_neverball_rs::{entities::FullGame, sol::Sol};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let sol = Sol::from_bytes(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut game = FullGame::new(sol);
    let mut camera = CameraRig::new(game.ball.position);
    let bytes = std::fs::read(&args[2]).unwrap();
    let mut at = 24;
    for _ in 0..4 {
        while bytes[at] != 0 {
            at += 1;
        }
        at += 1;
    }
    at += 24;
    let mut reference = Vec3::ZERO;
    let mut eye = Vec3::ZERO;
    let mut batch = 0;
    let mut count = 0;
    let mut max_position = 0f32;
    let mut max_eye = 0f32;
    let mut max_tilt = 0f32;
    let mut sum = 0.;
    let mut tilt = Vec2::ZERO;
    let mut samples = Vec::new();
    let mut collisions = 0;
    let mut events = Vec::new();
    while at + 3 <= bytes.len() {
        let cmd = bytes[at];
        let len = u16::from_le_bytes(bytes[at + 1..at + 3].try_into().unwrap()) as usize;
        at += 3;
        if at + len > bytes.len() {
            break;
        }
        let q = &bytes[at..at + len];
        at += len;
        let f = || {
            q.chunks_exact(4)
                .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                .collect::<Vec<_>>()
        };
        match cmd {
            5 => {
                let v = f();
                tilt = vec2(v[1], v[0]);
            }
            22 => reference = Vec3::from_array(f().try_into().unwrap()),
            25 => eye = Vec3::from_array(f().try_into().unwrap()),
            1 => {
                // SDL observed event times3000/6000/9000/11000ms translate to
                // batch44/314/584/764 at90Hz. This is one constant clock offset.
                if batch > 0 {
                    let dt = 1. / 90.;
                    let input = if (44..314).contains(&batch) {
                        vec2(0., 1.)
                    } else if (584..764).contains(&batch) {
                        vec2(0., -1.)
                    } else {
                        Vec2::ZERO
                    };
                    let report = game.step(input, camera.right, camera.back, dt);
                    collisions += report.collisions;
                    camera.step(game.ball.position, game.ball.velocity, 0., dt);
                    let error = game.ball.position.distance(reference);
                    let ce = camera.eye.distance(eye);
                    max_position = max_position.max(error);
                    max_eye = max_eye.max(ce);
                    max_tilt = max_tilt.max(game.tilt.distance(tilt));
                    sum += error;
                    count += 1;
                    if !game.events.is_empty() {
                        events.push(
                            serde_json::json!({"batch":batch,"events":format!("{:?}",game.events)}),
                        );
                    }
                    if batch % 100 == 0 || batch < 5 {
                        samples.push(serde_json::json!({"batch":batch,"position_error":error,"eye_error":ce,"rust":game.ball.position.to_array(),"reference":reference.to_array()}));
                    }
                }
                batch += 1;
            }
            _ => {}
        }
    }
    println!(
        "{}",
        serde_json::json!({"provenance":"Actual reference raw SDL event schedule; constant clock-origin alignment from first key transition. FullGame+CameraRig uses compiled SOL initial state and raw keys only. No replay position/velocity/tilt/camera/mover state injected.","samples":count,"max_position_error":max_position,"mean_position_error":sum/count as f32,"max_eye_error":max_eye,"max_tilt_error":max_tilt,"collisions":collisions,"events":events,"selected":samples})
    );
}
