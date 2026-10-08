// SPDX-License-Identifier: GPL-3.0-or-later
//! Independent raw-key course holdout. Reference state is measured only, never applied.
use macroquad::prelude::*;
use reagent_neverball_rs::{entities::FullGame, replay::OriginalReplay, sol::Sol, Outcome};
use std::collections::BTreeMap;
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let sol = Sol::from_bytes(&std::fs::read(&args[1]).unwrap()).unwrap();
    let reference = OriginalReplay::from_nbr(&std::fs::read(&args[2]).unwrap()).unwrap();
    let ledger: Vec<(u32, i32, bool)> = std::fs::read_to_string(&args[3])
        .unwrap()
        .lines()
        .skip(1)
        .map(|l| {
            let a: Vec<_> = l.split(',').collect();
            (a[0].parse().unwrap(), a[2].parse().unwrap(), a[3] == "1")
        })
        .collect();
    let features = serde_json::json!({"paths":sol.paths.len(),"switches":sol.switches.len(),"jumps":sol.jumps.len(),"size_items":sol.items.iter().filter(|i|i.kind==2||i.kind==3).count()});
    let mut r_position = sol.balls[0][..3].to_vec();
    let mut r_radius = sol.balls[0][3];
    let mut r_coins = 0;
    let mut r_tilt = [0f32; 2];
    let mut g = FullGame::new(sol);
    let mut cursor = 0;
    let mut mask = 0u8;
    let keys = [1073741903, 1073741904, 1073741906, 1073741905];
    let mut held = Vec2::ZERO;
    let mut dt = 1f32 / 90.;
    let mut ref_status = 0;
    let mut ref_commands = BTreeMap::<u8, u32>::new();
    let mut events = Vec::new();
    let mut frames = Vec::new();
    let mut max = 0f32;
    let mut sum = 0f64;
    let mut playing_max = 0f32;
    let mut playing_count = 0;
    let mut playing_sum = 0f64;
    let mut radius_max = 0f32;
    let mut tilt_max = 0f32;
    let mut coin_mismatches = 0;
    let mut first_divergence = None;
    let mut collisions = 0;
    let mut rust_terminal = None;
    let mut ref_terminal = None;
    for (i, batch) in reference.batches.iter().enumerate() {
        let logical_frame = 151 + (i as u32 * 2).saturating_sub(1) / 3;
        while cursor < ledger.len() && ledger[cursor].0 <= logical_frame {
            let (_, key, down) = ledger[cursor];
            if let Some(bit) = keys.iter().position(|&k| k == key) {
                if down {
                    mask |= 1 << bit
                } else {
                    mask &= !(1 << bit)
                }
            }
            cursor += 1;
        }
        let was_playing = g.outcome == Outcome::Playing;
        let ref_was_playing = ref_status == 0;
        for c in &batch.commands {
            match c.kind {
                22 => {
                    r_position = c
                        .payload
                        .chunks_exact(4)
                        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                        .collect()
                }
                19 => r_radius = f32::from_le_bytes(c.payload[..4].try_into().unwrap()),
                9 => r_coins = i32::from_le_bytes(c.payload[..4].try_into().unwrap()),
                5 => {
                    r_tilt = [
                        f32::from_le_bytes(c.payload[..4].try_into().unwrap()),
                        f32::from_le_bytes(c.payload[4..8].try_into().unwrap()),
                    ];
                }
                _ => {}
            }
            *ref_commands.entry(c.kind).or_default() += 1;
            if c.kind == 18 {
                dt = 1. / i32::from_le_bytes(c.payload[..4].try_into().unwrap()) as f32;
            }
            if c.kind == 8 {
                ref_status = i32::from_le_bytes(c.payload[..4].try_into().unwrap());
                if ref_terminal.is_none() {
                    ref_terminal = Some(i);
                }
            }
        }
        if i > 0 {
            if was_playing {
                held = vec2(
                    f32::from(mask & 1 != 0) - f32::from(mask & 2 != 0),
                    f32::from(mask & 4 != 0) - f32::from(mask & 8 != 0),
                );
                let r = g.step(held, Vec3::X, Vec3::Z, dt);
                collisions += r.collisions;
            } else {
                g.step_result(held, Vec3::X, Vec3::Z, dt);
            }
            if !g.events.is_empty() {
                events.push(serde_json::json!({"batch":i,"events":format!("{:?}",g.events)}));
            }
        }
        if !matches!(g.outcome, Outcome::Playing) && rust_terminal.is_none() {
            rust_terminal = Some(i);
        }
        let error = g
            .ball
            .position
            .distance(Vec3::from_array(r_position.clone().try_into().unwrap()));
        max = max.max(error);
        sum += error as f64;
        if was_playing && ref_was_playing {
            playing_max = playing_max.max(error);
            playing_sum += error as f64;
            playing_count += 1;
        }
        radius_max = radius_max.max((g.ball.radius - r_radius).abs());
        tilt_max = tilt_max.max(g.tilt.distance(vec2(r_tilt[1], r_tilt[0])));
        if g.coins != r_coins {
            coin_mismatches += 1;
        }
        if error > 0.01 && first_divergence.is_none() {
            first_divergence = Some(i);
        }
        frames.push(serde_json::json!({"batch":i,"logical_frame":logical_frame,"error":error,"rust":g.ball.position.to_array(),"velocity":g.ball.velocity.to_array(),"movers":g.paths.movers.iter().map(|m|(m.elapsed,m.path)).collect::<Vec<_>>(),"reference":r_position,"radius":g.ball.radius,"reference_radius":r_radius,"rust_coins":g.coins,"reference_coins":r_coins,"rust_status":format!("{:?}",g.outcome),"reference_status":ref_status}));
    }
    println!(
        "{}",
        serde_json::json!({"provenance":"Independent FullGame from compiled SOL initial state, exact delivered SDL raw keys only. Fixed clock origin frame151,60Hz holds across90Hz updates. Camera3 selected before arrow inputs; no replay state or positions applied. Jump batches also advance1/90 even when NBR emits no mover-time command30.","features":features,"batches":reference.batches.len(),"max_position_error":max,"mean_position_error":sum/reference.batches.len() as f64,"playing_max_position_error":playing_max,"playing_mean_position_error":playing_sum/playing_count as f64,"playing_batches":playing_count,"max_radius_error":radius_max,"max_tilt_error":tilt_max,"coin_mismatch_batches":coin_mismatches,"first_position_error_over_0_01":first_divergence,"rust_terminal_batch":rust_terminal,"reference_terminal_batch":ref_terminal,"rust_outcome":format!("{:?}",g.outcome),"rust_coins":g.coins,"collisions":collisions,"reference_command_counts":ref_commands,"rust_events":events,"frames":frames})
    );
}
