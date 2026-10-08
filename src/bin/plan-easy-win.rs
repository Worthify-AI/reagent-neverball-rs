// SPDX-License-Identifier: GPL-3.0-or-later
//! Offline arrow-key route planner, using reconstruction state only.
use macroquad::prelude::*;
use reagent_neverball_rs::{entities::FullGame, sol::Sol, Outcome};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let sol = Sol::from_bytes(&std::fs::read(&args[1]).unwrap()).unwrap();
    let vmax = args
        .get(3)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.);
    let kp = args
        .get(4)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.5);
    let damping = args
        .get(5)
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(0.25);
    let mut g = FullGame::new(sol);
    let route = [10usize, 9, 8, 16, 14, 17, 15, 7, 6, 5];
    let mut stage = 0;
    let mut mask = 0u8;
    let mut schedule = String::from("30 13 1\n31 13 0\n160 51 1\n161 51 0\n");
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let mut acc = 0f64;
    let mut last_frame = 0;
    let keys = [1073741903, 1073741904, 1073741906, 1073741905];
    for frame in 151..5400u32 {
        while stage < route.len() && g.collected[route[stage]] {
            events.push(serde_json::json!({"frame":frame,"stage":stage,"coins":g.coins,"position":g.ball.position.to_array()}));
            stage += 1;
        }
        let target = if stage < route.len() {
            let p = g.sol.items[route[stage]].position;
            vec2(p[0], p[2])
        } else {
            let p = g.sol.goals[0];
            vec2(p[0], p[2])
        };
        let pos = vec2(g.ball.position.x, g.ball.position.z);
        let velocity = vec2(g.ball.velocity.x, g.ball.velocity.z);
        let desired = ((target - pos) * kp).clamp_length_max(vmax);
        let tilt_predict = vec2(g.tilt.x, -g.tilt.y) * 3.35 / 20. * damping;
        let error = desired - velocity - tilt_predict;
        let threshold = 0.035;
        let mut next = 0u8;
        if frame >= 180 {
            if error.x > threshold {
                next |= 1
            } else if error.x < -threshold {
                next |= 2
            }
            if error.y < -threshold {
                next |= 4
            } else if error.y > threshold {
                next |= 8
            }
        }
        if next != mask {
            for (bit, key) in keys.iter().enumerate() {
                if (next ^ mask) & (1 << bit) != 0 {
                    schedule.push_str(&format!(
                        "{frame} {} {}\n",
                        key,
                        if next & (1 << bit) != 0 { 1 } else { 0 }
                    ));
                }
            }
        }
        mask = next;
        let input = vec2(
            f32::from(mask & 1 != 0) - f32::from(mask & 2 != 0),
            f32::from(mask & 4 != 0) - f32::from(mask & 8 != 0),
        );
        acc += 1. / 60.;
        while acc + 1e-10 >= 1. / 90. {
            g.step(input, Vec3::X, Vec3::Z, 1. / 90.);
            acc -= 1. / 90.;
        }
        if frame % 30 == 0 {
            samples.push(serde_json::json!({"frame":frame,"position":g.ball.position.to_array(),"velocity":g.ball.velocity.to_array(),"coins":g.coins,"stage":stage,"mask":mask}));
        }
        last_frame = frame;
        if g.outcome != Outcome::Playing {
            break;
        }
    }
    for (bit, key) in keys.iter().enumerate() {
        if mask & (1 << bit) != 0 {
            schedule.push_str(&format!("{} {} 0\n", last_frame + 1, key));
        }
    }
    let result = serde_json::json!({"outcome":format!("{:?}",g.outcome),"elapsed":g.elapsed,"coins":g.coins,"position":g.ball.position.to_array(),"stage":stage,"last_frame":last_frame,"input_events":schedule.lines().count(),"vmax":vmax,"kp":kp,"damping":damping,"events":events,"samples":samples});
    std::fs::write(&args[2], schedule).unwrap();
    std::fs::write(
        format!("{}.json", args[2]),
        serde_json::to_string_pretty(&result).unwrap(),
    )
    .unwrap();
    println!(
        "{}",
        serde_json::json!({"outcome":result["outcome"],"elapsed":g.elapsed,"coins":g.coins,"stage":stage,"position":result["position"],"input_events":result["input_events"]})
    );
}
