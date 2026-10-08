// SPDX-License-Identifier: GPL-3.0-or-later
// Differential harness: feeds recorded reference gravity frames into the reconstructed solver.
use macroquad::prelude::{vec3, Quat, Vec3};
use reagent_neverball_rs::{entities::FullGame, sol::Sol};
fn floats(q: &[u8]) -> Vec<f32> {
    q.chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect()
}
fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let sol = Sol::from_bytes(&std::fs::read(&args[1]).unwrap()).unwrap();
    let mut game = FullGame::new(sol);
    let bytes = std::fs::read(&args[2]).unwrap();
    let mut at = 24;
    for _ in 0..4 {
        while bytes[at] != 0 {
            at += 1;
        }
        at += 1;
    }
    at += 24;
    let mut dt = 0.;
    let mut tilt = [0.; 2];
    let mut basis = [1., 0., 0., 0., 0., 1.];
    let mut reference = Vec3::ZERO;
    let mut samples = 0;
    let mut max_error = 0f32;
    let mut outer = [1., 0., 0., 0., 1., 0.];
    let mut inner = outer;
    let mut max_outer = 0f32;
    let mut max_outer_inverse = 0f32;
    let mut max_inner = 0f32;
    let mut sum = 0.;
    let mut errors = Vec::new();
    let mut initial = true;
    while at < bytes.len() {
        let command = bytes[at];
        let length = u16::from_le_bytes(bytes[at + 1..at + 3].try_into().unwrap()) as usize;
        at += 3;
        let q = &bytes[at..at + length];
        at += length;
        match command {
            5 => tilt.copy_from_slice(&floats(q)),
            32 => basis.copy_from_slice(&floats(q)),
            30 => dt += floats(q)[0],
            23 => outer.copy_from_slice(&floats(q)),
            24 => inner.copy_from_slice(&floats(q)),
            22 => reference = Vec3::from_array(floats(q).try_into().unwrap()),
            1 => {
                if initial {
                    game.ball.position = reference;
                    initial = false;
                } else if dt > 0. {
                    let right = vec3(basis[0], basis[1], basis[2]);
                    let back = vec3(basis[3], basis[4], basis[5]);
                    let rotation = Quat::from_axis_angle(back, tilt[1].to_radians())
                        * Quat::from_axis_angle(right, tilt[0].to_radians());
                    let gravity = rotation * vec3(0., -9.8, 0.);
                    let previous_velocity = game.ball.velocity;
                    game.paths
                        .step_ball(&game.sol, &game.world, &mut game.ball, gravity, dt);
                    game.ball.update_inner(previous_velocity, gravity, dt);

                    let error = game.ball.position.distance(reference);
                    max_error = max_error.max(error);
                    sum += error;
                    samples += 1;
                    let ox = Vec3::from_slice(&outer[..3]);
                    let oy = Vec3::from_slice(&outer[3..]);
                    let q = game.ball.orientation;
                    let oe = (q * Vec3::X - ox).length().max((q * Vec3::Y - oy).length());
                    let ie = (q.conjugate() * Vec3::X - ox)
                        .length()
                        .max((q.conjugate() * Vec3::Y - oy).length());
                    let iq = game.ball.inner_orientation;
                    let ine = (iq * Vec3::X - Vec3::from_slice(&inner[..3]))
                        .length()
                        .max((iq * Vec3::Y - Vec3::from_slice(&inner[3..])).length());
                    max_inner = max_inner.max(ine);
                    max_outer = max_outer.max(oe);
                    max_outer_inverse = max_outer_inverse.max(ie);
                    if samples < 10 || samples % 100 == 0 {
                        errors.push(serde_json::json!({"sample":samples,"error":error,"reference":reference.to_array(),"rust":game.ball.position.to_array(),"tilt":tilt,"dt":dt,"inner_error":ine,"inner_rust_y":(iq*Vec3::Y).to_array(),"outer_error":oe,"inverse_error":ie,"outer_reference":outer,"inner_reference":inner,"outer_rust":(q*Vec3::X).to_array()}));
                    }
                }
                dt = 0.;
            }
            _ => {}
        }
    }
    println!(
        "{}",
        serde_json::json!({"max_inner_basis_error":max_inner,"max_outer_basis_error":max_outer,"max_inverse_basis_error":max_outer_inverse,"samples":samples,"max_position_error":max_error,"mean_position_error":sum/samples as f32,"selected":errors})
    );
}
