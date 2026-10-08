// SPDX-License-Identifier: GPL-3.0-or-later
//! Result-animation extension of the same-key holdout; no replay state applied.
use macroquad::prelude::*;
use reagent_neverball_rs::{entities::FullGame, replay::OriginalReplay, sol::Sol};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let sol = Sol::from_bytes(&std::fs::read(&args[1]).unwrap()).unwrap();
    let original = OriginalReplay::from_nbr(&std::fs::read(&args[2]).unwrap()).unwrap();
    let script: Vec<(u32, i32, bool)> = std::fs::read_to_string(&args[3])
        .unwrap()
        .lines()
        .map(|l| {
            let a: Vec<_> = l.split_whitespace().collect();
            (a[0].parse().unwrap(), a[1].parse().unwrap(), a[2] == "1")
        })
        .collect();
    let origin = args
        .get(4)
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(151.);
    let mut cursor = 0;
    let mut mask = 0u8;
    let mut g = FullGame::new(sol);
    let mut max = 0f32;
    let mut sum = 0f32;
    let mut samples = Vec::new();
    let mut events = Vec::new();
    let mut first_divergence = None;
    let mut coins_mismatch = 0;
    let mut terminal_input = Vec2::ZERO;
    let mut physics_step = 0u32;
    let mut playing_max = 0f32;
    let mut playing_sum = 0f64;
    let mut playing_count = 0u32;
    let mut won_batch = None;
    let keys = [1073741903, 1073741904, 1073741906, 1073741905];
    for (i, batch) in original.batches.iter().enumerate() {
        let dt: f32 = if i > 0 { 1. / 90. } else { 0. };
        if dt > 0. {
            physics_step += 1;
        }
        let logical_frame = origin as u32 + physics_step.saturating_mul(2).saturating_sub(1) / 3;
        while cursor < script.len() && script[cursor].0 <= logical_frame {
            let (_, key, down) = script[cursor];
            if let Some(bit) = keys.iter().position(|&k| k == key) {
                if down {
                    mask |= 1 << bit
                } else {
                    mask &= !(1 << bit)
                }
            }
            cursor += 1;
        }
        let was_playing = g.outcome == reagent_neverball_rs::Outcome::Playing;
        if dt > 0. {
            let input = vec2(
                f32::from(mask & 1 != 0) - f32::from(mask & 2 != 0),
                f32::from(mask & 4 != 0) - f32::from(mask & 8 != 0),
            );
            if was_playing {
                terminal_input = input;
                g.step(input, Vec3::X, Vec3::Z, dt);
            } else {
                g.step_result(terminal_input, Vec3::X, Vec3::Z, dt);
            }
            if !g.events.is_empty() {
                events.push(serde_json::json!({"batch":i,"time":g.elapsed,"events":format!("{:?}",g.events)}));
            }
        }
        let reference = &original.replay.frames[i];
        let error = g
            .ball
            .position
            .distance(Vec3::from_array(reference.position));
        max = max.max(error);
        sum += error;
        if was_playing {
            playing_max = playing_max.max(error);
            playing_sum += error as f64;
            playing_count += 1;
        }
        if won_batch.is_none() && g.outcome == reagent_neverball_rs::Outcome::Won {
            won_batch = Some(i);
        }
        if error > 0.05 && first_divergence.is_none() {
            first_divergence = Some(
                serde_json::json!({"batch":i,"time":batch.at_seconds,"error":error,"rust":g.ball.position.to_array(),"reference":reference.position}),
            );
        }
        if g.coins != reference.coins {
            coins_mismatch += 1;
        }
        if i % 90 == 0 || i + 1 == original.batches.len() {
            samples.push(serde_json::json!({"batch":i,"time":batch.at_seconds,"error":error,"rust":g.ball.position.to_array(),"reference":reference.position,"coins":g.coins,"reference_coins":reference.coins}));
        }
    }
    println!(
        "{}",
        serde_json::json!({"method":"Fixed90Hz independent simulation with exact raw arrow-key schedule and compiled level initialstate; no recorded position/camera/tilt/state applied.","clock_origin_frame":origin,"batches":original.batches.len(),"input_clock":"60Hz frame holds across alternating one/two 90Hz steps; frame151 has step1","playing_max_position_error":playing_max,"playing_mean_position_error":playing_sum/playing_count as f64,"playing_batches":playing_count,"rust_won_batch":won_batch,"reference_status":original.header.status,"reference_final_coins":original.header.coins,"rust_outcome":format!("{:?}",g.outcome),"rust_coins":g.coins,"rust_elapsed":g.elapsed,"max_position_error":max,"mean_position_error":sum/original.batches.len() as f32,"coins_mismatch_batches":coins_mismatch,"first_divergence":first_divergence,"samples":samples,"events":events})
    );
}
