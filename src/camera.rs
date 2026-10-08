// SPDX-License-Identifier: GPL-3.0-or-later
// Independently reconstructed from reference ELF FUN_0001abc0 and replay commands
// 25/26/27. No original engine implementation source was used.
use macroquad::prelude::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraMode {
    Chase,
    Lazy,
    Manual,
}
#[derive(Clone, Debug)]
pub struct CameraRig {
    pub mode: CameraMode,
    pub eye: Vec3,
    pub target: Vec3,
    pub right: Vec3,
    pub back: Vec3,
    pub yaw: f32,
    distance: f32,
    rotation_time: f32,
    recovery: f32,
}
impl CameraRig {
    pub fn new(position: Vec3) -> Self {
        Self {
            mode: CameraMode::Chase,
            eye: position + vec3(0., 0.75, 2.),
            target: position + Vec3::Y * 0.25,
            right: Vec3::X,
            back: Vec3::Z,
            yaw: 0.,
            distance: 1.,
            rotation_time: 0.,
            recovery: 0.,
        }
    }
    pub fn reset(&mut self, position: Vec3) {
        let mode = self.mode;
        *self = Self::new(position);
        self.mode = mode;
    }
    /// Binary FUN_0001d930 interpolates default view toward compiled SOL views
    /// with a squared transition parameter. The caller supplies transition time.
    pub fn preview(&mut self, position: Vec3, views: &[[f32; 6]], amount: f32) {
        let default_eye = position + vec3(0., 0.75, 2.);
        let default_target = position + Vec3::Y * 0.25;
        if let Some(v) = views.first() {
            let t = amount * amount;
            self.eye = default_eye.lerp(vec3(v[0], v[1], v[2]), t);
            self.target = default_target.lerp(vec3(v[3], v[4], v[5]), t);
        } else {
            self.eye = default_eye;
            self.target = default_target;
        }
        self.right = Vec3::Y.cross(self.eye - self.target).normalize_or_zero();
        self.back = self.right.cross(Vec3::Y).normalize_or_zero();
        self.yaw = self.back.x.atan2(self.back.z);
    }
    pub fn toggle(&mut self) {
        self.mode = if self.mode == CameraMode::Manual {
            CameraMode::Chase
        } else {
            CameraMode::Manual
        };
    }
    /// Executable FUN1abc0 narrows target elevation through a jump and shifts
    /// the previous eye by the teleport displacement before normal tracking.
    #[allow(clippy::too_many_arguments)]
    pub fn step_jump(
        &mut self,
        position: Vec3,
        velocity: Vec3,
        rotation: f32,
        dt: f32,
        jump_elapsed: Option<f32>,
        teleport_delta: Vec3,
    ) {
        self.eye += teleport_delta;
        self.step(position, velocity, rotation, dt);
        if let Some(elapsed) = jump_elapsed {
            self.target = position + Vec3::Y * (0.25 * (2. * (elapsed - 0.5).abs()));
        }
    }
    /// `rotation` is the reference normalized rotation control: slow ±1.5,
    /// fast ±3; the binary multiplies this by 90 degrees per second.
    pub fn step(&mut self, position: Vec3, velocity: Vec3, rotation: f32, dt: f32) {
        let angle = rotation * 90. * dt;
        let mut direction = self.back;
        let horizontal = vec3(velocity.x, 0., velocity.z);
        if angle == 0. {
            if self.rotation_time < 0. {
                self.recovery = (-self.rotation_time).clamp(0.2, 1.);
                self.rotation_time = 0.;
            }
            self.rotation_time += dt;
            if self.mode != CameraMode::Manual {
                direction = (self.eye - position).normalize_or_zero();
                let blend = if self.recovery == 0. {
                    1.
                } else {
                    (self.rotation_time / self.recovery).powi(3).clamp(0., 1.)
                };
                let speed = if self.mode == CameraMode::Chase {
                    0.25
                } else {
                    0.
                };
                direction -= horizontal * (horizontal.length() * speed * blend * dt);
            } else {
                direction = vec3(self.yaw.sin(), 0., self.yaw.cos());
            }
        } else {
            if self.rotation_time > 0. {
                self.recovery = 0.;
                self.rotation_time = 0.;
            }
            self.rotation_time -= dt;
            if self.mode == CameraMode::Manual {
                direction = vec3(self.yaw.sin(), 0., self.yaw.cos());
            }
            direction = Quat::from_rotation_y(angle.to_radians()) * direction;
        }
        self.right = Vec3::Y.cross(direction).normalize_or_zero();
        self.back = self.right.cross(Vec3::Y).normalize_or_zero();
        self.distance =
            (self.distance + dt * (1. - horizontal.dot(self.back) / 10. - self.distance)).max(0.5);
        self.eye = position + (Vec3::Y * 0.75 + self.back * 2.) * self.distance;
        self.target = position + Vec3::Y * 0.25;
        self.yaw = self.back.x.atan2(self.back.z);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jump_target_dips_and_recovers() {
        for (elapsed, height) in [(0., 0.25), (0.25, 0.125), (0.5, 0.), (0.75, 0.125)] {
            let mut camera = CameraRig::new(Vec3::ZERO);
            camera.step_jump(
                Vec3::ZERO,
                Vec3::ZERO,
                0.,
                1. / 90.,
                Some(elapsed),
                Vec3::ZERO,
            );
            assert_eq!(camera.target.y, height);
        }
    }
    #[test]
    fn manual_rotation_and_reset() {
        let mut c = CameraRig::new(Vec3::ZERO);
        c.mode = CameraMode::Manual;
        c.step(Vec3::ZERO, Vec3::ZERO, 1.5, 1. / 90.);
        assert!((c.yaw.to_degrees() - 1.5).abs() < 1e-5);
        c.reset(Vec3::ONE);
        assert_eq!(c.mode, CameraMode::Manual);
        assert_eq!(c.eye, vec3(1., 1.75, 3.));
    }
    #[test]
    fn modes_respond_differently_to_sideways_travel() {
        let mut a = CameraRig::new(Vec3::ZERO);
        let mut b = a.clone();
        let mut c = a.clone();
        b.mode = CameraMode::Lazy;
        c.mode = CameraMode::Manual;
        for i in 1..91 {
            let p = vec3(i as f32 / 90., 0., 0.);
            for camera in [&mut a, &mut b, &mut c] {
                camera.step(p, Vec3::X, 0., 1. / 90.);
            }
        }
        assert!(a.yaw < b.yaw);
        assert!(b.yaw < 0.);
        assert_eq!(c.yaw, 0.);
    }
}
