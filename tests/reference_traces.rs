// SPDX-License-Identifier: GPL-3.0-or-later
// Expected positions originate in licensed reference runtime recordings.
use macroquad::prelude::vec2;
use reagent_neverball_rs::{Course, Game, STEP};
use serde_json::Value;
use std::collections::BTreeSet;
#[test]
fn seven_recorded_reference_trajectories() {
    for name in [
        "construction",
        "holdout-right",
        "holdout-left",
        "holdout-diagonal",
        "holdout-short-pulses",
        "final-holdout-left-forward",
        "final-holdout-long-forward",
    ] {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/traces");
        let rows: Vec<Value> = serde_json::from_str(
            &std::fs::read_to_string(root.join(format!("{name}.json"))).unwrap(),
        )
        .unwrap();
        let script = std::fs::read_to_string(root.join(format!("{name}.txt"))).unwrap();
        let events: Vec<(f64, i32, bool)> = script
            .lines()
            .map(|l| {
                let v: Vec<_> = l.split_whitespace().collect();
                (
                    v[0].parse::<f64>().unwrap() / 60. - 2.5222222222,
                    v[1].parse().unwrap(),
                    v[2] == "1",
                )
            })
            .collect();
        let mut game = Game::new(Course::reference());
        let mut event = 0;
        let mut held = BTreeSet::new();
        let mut last = 0.;
        let mut max = 0f32;
        let mut n = 0;
        for row in rows {
            let time = row["t"].as_f64().unwrap();
            if time == 0. {
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
                (held.contains(&1073741903) as u8 as f32)
                    - (held.contains(&1073741904) as u8 as f32),
                (held.contains(&1073741906) as u8 as f32)
                    - (held.contains(&1073741905) as u8 as f32),
            );
            game.step(input, STEP);
            last = time;
            let p = &row["position"];
            let expected = macroquad::prelude::vec3(
                p[0].as_f64().unwrap() as f32,
                p[1].as_f64().unwrap() as f32,
                p[2].as_f64().unwrap() as f32,
            );
            max = max.max(game.position.distance(expected));
            n += 1;
        }
        assert!(n > 1000, "{name}: incomplete recording");
        assert!(
            max < 0.10,
            "{name}: maximum error {max} exceeds the declared 0.10 world-unit boundary"
        );
    }
}
