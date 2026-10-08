// SPDX-License-Identifier: GPL-3.0-or-later
#[path = "../src/camera.rs"]
#[allow(dead_code)]
mod camera;
use camera::{CameraMode, CameraRig};
use macroquad::prelude::*;
use serde::Deserialize;
#[derive(Deserialize)]
struct Trace {
    initial_position: [f32; 3],
    samples: Vec<Sample>,
}
#[derive(Deserialize)]
struct Sample {
    dt: f32,
    position: [f32; 3],
    estimated_velocity: [f32; 3],
    eye: [f32; 3],
    rotation: f32,
    mode: u8,
}
#[test]
fn recorded_camera_modes_and_rotation() {
    let trace: Trace =
        serde_json::from_str(include_str!("../fixtures/camera-reference.json")).unwrap();
    let mut camera = CameraRig::new(Vec3::from_array(trace.initial_position));
    let mut max = 0f32;
    let mut total = 0.;
    for s in &trace.samples {
        camera.mode = match s.mode {
            0 => CameraMode::Chase,
            1 => CameraMode::Lazy,
            _ => CameraMode::Manual,
        };
        camera.step(
            Vec3::from_array(s.position),
            Vec3::from_array(s.estimated_velocity),
            s.rotation,
            s.dt,
        );
        let error = camera.eye.distance(Vec3::from_array(s.eye));
        max = max.max(error);
        total += error;
    }
    eprintln!(
        "camera replay samples={}, max eye error={}, mean={}",
        trace.samples.len(),
        max,
        total / trace.samples.len() as f32
    );
    // Velocity is estimated from replay positions, not the inaccessible live velocity.
    assert_eq!(trace.samples.len(), 1123);
    assert!(max < 0.006, "camera eye error {max}");
}
