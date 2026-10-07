// SPDX-License-Identifier: GPL-3.0-or-later
use macroquad::prelude::vec2;
use reagent_neverball_rs::{Course, Game, STEP};
use serde_json::{json, Value};
use std::collections::BTreeSet;
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rows: Vec<Value> =
        serde_json::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let script = std::fs::read_to_string(&args[2]).unwrap();
    let events: Vec<(f64, i32, bool)> = script
        .lines()
        .filter_map(|l| {
            let v: Vec<_> = l.split_whitespace().collect();
            if v.len() != 3 {
                return None;
            }
            Some((
                v[0].parse::<f64>().unwrap() / 60.0 - 2.5222222222,
                v[1].parse().unwrap(),
                v[2] == "1",
            ))
        })
        .collect();
    let mut held = BTreeSet::new();
    let mut event = 0;
    let mut g = Game::new(Course::reference());
    let mut n = 0;
    let mut max = 0f32;
    let mut sums = 0f64;
    let mut states = Vec::new();
    let mut last = 0f64;
    for row in rows {
        let t = row["t"].as_f64().unwrap();
        if t == 0. {
            continue;
        }
        while event < events.len() && events[event].0 <= last + 1e-6 {
            let (_, key, down) = events[event];
            if down {
                held.insert(key);
            } else {
                held.remove(&key);
            }
            event += 1;
        }
        let input = vec2(
            (held.contains(&1073741903) as u8 as f32) - (held.contains(&1073741904) as u8 as f32),
            (held.contains(&1073741906) as u8 as f32) - (held.contains(&1073741905) as u8 as f32),
        );
        g.step(input, STEP);
        last = t;
        let p = row.get("position").unwrap_or(&row["22"]);
        let diff = ((g.position.x - p[0].as_f64().unwrap() as f32).powi(2)
            + (g.position.y - p[1].as_f64().unwrap() as f32).powi(2)
            + (g.position.z - p[2].as_f64().unwrap() as f32).powi(2))
        .sqrt();
        max = max.max(diff);
        sums += f64::from(diff) * f64::from(diff);
        n += 1;
        states.push(json!({"t":t,"position":g.position.to_array(),"tilt":g.tilt.to_array(),"error":diff,"coins":g.coins(),"outcome":format!("{:?}",g.outcome)}));
    }
    println!(
        "{}",
        json!({"samples":n,"max_position_error":max,"rms_position_error":(sums/n as f64).sqrt(),"final_position":g.position.to_array(),"coins":g.coins(),"outcome":format!("{:?}",g.outcome),"clock_offset_seconds":2.5222222222,"states":states})
    );
}
