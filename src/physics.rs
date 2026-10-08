// SPDX-License-Identifier: GPL-3.0-or-later
//! Executable-derived sphere/convex collision. See physics-task.json for provenance.
use macroquad::prelude::{Quat, Vec3};
// FUN10160 rotates via two quaternion products, including non-unit rounding.
fn quaternion_product(a: Quat, b: Quat) -> Quat {
    Quat::from_xyzw(
        a.w * b.x + a.x * b.w + a.y * b.z - a.z * b.y,
        a.w * b.y - a.x * b.z + a.y * b.w + a.z * b.x,
        a.w * b.z + a.x * b.y - a.y * b.x + a.z * b.w,
        a.w * b.w - a.x * b.x - a.y * b.y - a.z * b.z,
    )
}
fn rotate_vector(q: Quat, v: Vec3) -> Vec3 {
    let r = quaternion_product(
        quaternion_product(q, Quat::from_xyzw(v.x, v.y, v.z, 0.)),
        q.conjugate(),
    );
    Vec3::new(r.x, r.y, r.z)
}
/// Quaternion interpolation preserves FUN35600's float rounding and endpoint values.
pub(crate) fn path_slerp(a: Quat, b: Quat, t: f32) -> Quat {
    if t <= 0. {
        return a;
    }
    if t >= 1. {
        return b;
    }
    let dot = a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w;
    let sign = if dot < 0. { -1. } else { 1. };
    let dot = dot.abs();
    let (wa, wb) = if 1. - dot >= 0.00001 {
        let angle = (dot as f64).acos() as f32;
        let denominator = (angle as f64).sin() as f32;
        (
            (((1. - t) * angle) as f64).sin() as f32 / denominator,
            ((t * angle) as f64).sin() as f32 / denominator * sign,
        )
    } else {
        (1. - t, t)
    };
    let x = a.x * wa + b.x * wb;
    let y = a.y * wa + b.y * wb;
    let z = a.z * wa + b.z * wb;
    let w = a.w * wa + b.w * wb;
    let length = (x * x + y * y + z * z + w * w).sqrt();
    if length > 0. {
        Quat::from_xyzw(x / length, y / length, z / length, w / length)
    } else {
        Quat::IDENTITY
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BodyPose {
    pub translation: Vec3,
    pub rotation: Quat,
}
impl Default for BodyPose {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
        }
    }
}
impl BodyPose {
    pub fn point(self, p: Vec3) -> Vec3 {
        self.translation + rotate_vector(self.rotation, p)
    }
    pub fn inverse_point(self, p: Vec3) -> Vec3 {
        rotate_vector(self.rotation.conjugate(), p - self.translation)
    }
    pub fn interpolate(self, b: Self, t: f32) -> Self {
        Self {
            translation: self.translation.lerp(b.translation, t),
            rotation: path_slerp(self.rotation, b.rotation, t),
        }
    }
}
#[derive(Clone, Debug)]
pub struct Collider {
    pub body: usize,
    pub planes: Vec<(Vec3, f32)>,
    pub vertices: Vec<Vec3>,
    pub edges: Vec<(Vec3, Vec3)>,
    pub lo: Vec3,
    pub hi: Vec3,
}
#[derive(Clone, Copy, Debug)]
pub struct BallState {
    pub position: Vec3,
    pub velocity: Vec3,
    pub radius: f32,
    pub orientation: Quat,
    pub angular_velocity: Vec3,
    pub inner_orientation: Quat,
    pub inner_angular_velocity: Vec3,
}
impl BallState {
    pub fn new(position: Vec3, radius: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            radius,
            orientation: Quat::IDENTITY,
            angular_velocity: Vec3::ZERO,
            inner_orientation: Quat::IDENTITY,
            inner_angular_velocity: Vec3::ZERO,
        }
    }
    /// Executable FUN35ab0 secondary inner-ball update, once per full frame.
    /// The final correction preserves the binary's sequential component writes.
    pub fn update_inner(&mut self, previous_velocity: Vec3, gravity: Vec3, dt: f32) {
        if dt <= 0. {
            return;
        }
        let acceleration = ((self.velocity - previous_velocity) * 0.5 - gravity * dt) * (5. / dt);
        let lever = (self.inner_orientation * Vec3::Y) * (-self.radius);
        let torque = if lever.dot(acceleration).abs() > 0. {
            acceleration.cross(lever)
        } else {
            Vec3::ZERO
        };
        self.inner_angular_velocity = (self.inner_angular_velocity + torque * dt) * 0.995;
        Self::rotate_basis(&mut self.inner_orientation, self.inner_angular_velocity, dt);
        let up = self.inner_orientation * Vec3::Y;
        let back = self.inner_orientation * Vec3::Z;
        let projected = self.velocity + up * self.velocity.dot(up);
        let cross = projected.cross(back);
        let ty = cross.y * up.y;
        let tz = cross.z * up.z;
        let x = 2. * (cross.x * up.x + ty + tz) * up.x;
        let tx = up.x * x;
        let y = 2. * (ty + tx + tz) * up.y;
        let z = 2. * (y * up.y + tx + tz) * up.z;
        Self::rotate_basis(&mut self.inner_orientation, Vec3::new(x, y, z), dt);
    }
    fn rotate_basis(orientation: &mut Quat, velocity: Vec3, dt: f32) {
        let speed = velocity.length();
        if speed > 0. {
            *orientation =
                (Quat::from_axis_angle(velocity / speed, speed * dt) * *orientation).normalize();
        }
    }
    fn advance(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        let angle = self.angular_velocity.length() * dt;
        if angle > 0.0 {
            self.orientation = (Quat::from_axis_angle(self.angular_velocity.normalize(), angle)
                * self.orientation)
                .normalize();
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub time: f32,
    pub point: Vec3,
    pub normal: Vec3,
}
/// Smallest nonnegative root; overlapping spheres collide only while approaching.
fn sphere_time(offset: Vec3, velocity: Vec3, radius: f32) -> Option<f32> {
    let a = velocity.length_squared();
    if a == 0.0 {
        return None;
    }
    let dot = offset.dot(velocity);
    let b = dot + dot;
    let c = offset.length_squared() - radius * radius;
    let discriminant = b * b - (4.0 * a) * c;
    if discriminant < 0. {
        return None;
    }
    let root = (-b - discriminant.sqrt()) * 0.5 / a;
    (root >= 0.).then_some(root)
}
impl Collider {
    pub fn new(
        body: usize,
        planes: Vec<(Vec3, f32)>,
        vertices: Vec<Vec3>,
        edges: Vec<(Vec3, Vec3)>,
    ) -> Self {
        let mut lo = Vec3::splat(f32::INFINITY);
        let mut hi = Vec3::splat(f32::NEG_INFINITY);
        for &v in &vertices {
            lo = lo.min(v);
            hi = hi.max(v);
        }
        if vertices.is_empty() {
            lo = Vec3::splat(-1e6);
            hi = Vec3::splat(1e6);
        }
        Self {
            body,
            planes,
            vertices,
            edges,
            lo,
            hi,
        }
    }
    /// Sphere swept against face polygons, finite edge cylinders, and vertex spheres.
    /// Face containment is against all planes, preventing square expanded corners.
    pub fn sweep(&self, p: Vec3, v: Vec3, r: f32, limit: f32) -> Option<Hit> {
        self.sweep_translated(p, v, r, limit, Vec3::ZERO, Vec3::ZERO)
    }
    /// Binary's nonrotating path keeps world coordinates and subtracts plane
    /// offsets after dot products, avoiding cancellation from local transforms.
    pub fn sweep_translated(
        &self,
        p: Vec3,
        v: Vec3,
        r: f32,
        limit: f32,
        translation: Vec3,
        surface: Vec3,
    ) -> Option<Hit> {
        let local = p - translation;
        let relative = v - surface;
        let sweep_lo = local.min(local + relative * limit) - Vec3::splat(r);
        let sweep_hi = local.max(local + relative * limit) + Vec3::splat(r);
        if !sweep_hi.cmpge(self.lo).all() || !sweep_lo.cmple(self.hi).all() {
            return None;
        }
        let mut best: Option<Hit> = None;
        let mut offer = |time: f32, point: Vec3| {
            if time >= 0. && time < best.map_or(limit, |h| h.time) {
                let delta = p + v * time - point;
                if delta.length_squared() > 0. {
                    best = Some(Hit {
                        time,
                        point,
                        normal: delta / delta.length(),
                    });
                }
            }
        };
        for &vertex in &self.vertices {
            let point = vertex + translation;
            let offset = p - point;
            if offset.dot(relative) < 0. {
                if let Some(t) = sphere_time(offset, relative, r) {
                    offer(t, point + surface * t);
                }
            }
        }
        for &(a, b) in &self.edges {
            let edge = b - a;
            let length2 = edge.length_squared();
            if length2 == 0. {
                continue;
            }
            let offset = (p - translation) - a;
            let projection = offset.dot(edge);
            let perpendicular = edge * (-projection / length2) + offset;
            if perpendicular.length_squared() < r * r {
                if projection >= 0. && projection <= length2 && perpendicular.dot(relative) < 0. {
                    offer(0., p - (perpendicular / perpendicular.length()) * r);
                }
            } else {
                let speed = edge.dot(relative);
                let perpendicular_v = edge * (-speed / length2) + relative;
                if let Some(t) = sphere_time(perpendicular, perpendicular_v, r) {
                    let u = (speed * t + projection) / length2;
                    if u > 0. && u < 1. {
                        offer(t, edge * u + a + surface * t + translation);
                    }
                }
            }
        }
        for (index, &(n, d)) in self.planes.iter().enumerate() {
            let approach = n.dot(v) - n.dot(surface);
            if approach >= 0. {
                continue;
            }
            let offset = translation.dot(n);
            let absolute = p.dot(n);
            let t = (r + d + offset - absolute) / approach;
            let t = if t < 0. && (d + offset - absolute) / approach >= 0. {
                0.
            } else {
                t
            };
            if t < 0. || t >= limit {
                continue;
            }
            let point = v * t + p - n * r;
            if self.planes.iter().enumerate().all(|(other, &(pn, pd))| {
                other == index || (pn.dot(point) - pn.dot(translation)) - pn.dot(surface) * t <= pd
            }) {
                offer(t, point);
            }
        }

        best
    }
}
#[derive(Clone, Debug, Default)]
pub struct PhysicsWorld {
    pub colliders: Vec<Collider>,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct StepReport {
    pub collisions: usize,
    pub max_impact: f32,
    pub exhausted: bool,
}
/// Motion is advanced at every collision, matching executable path clocks.
pub trait BodyMotion {
    fn poses(&self, ahead: f32) -> Vec<BodyPose>;
    fn boundary(&self, dt: f32) -> f32 {
        dt
    }
    fn advance(&mut self, dt: f32);
    fn rotation_active(&self, body: usize, ahead: f32) -> bool {
        self.poses(0.)
            .get(body)
            .is_some_and(|p| p.rotation != Quat::IDENTITY)
            || self
                .poses(ahead)
                .get(body)
                .is_some_and(|p| p.rotation != Quat::IDENTITY)
    }
}
struct LinearMotion<'a> {
    start: &'a [BodyPose],
    end: &'a [BodyPose],
    duration: f32,
    elapsed: f32,
}
impl BodyMotion for LinearMotion<'_> {
    fn poses(&self, ahead: f32) -> Vec<BodyPose> {
        self.start
            .iter()
            .enumerate()
            .map(|(i, &a)| {
                a.interpolate(
                    self.end.get(i).copied().unwrap_or(a),
                    ((self.elapsed + ahead) / self.duration).clamp(0., 1.),
                )
            })
            .collect()
    }
    fn advance(&mut self, dt: f32) {
        self.elapsed += dt;
    }
}
impl PhysicsWorld {
    pub fn step(
        &self,
        ball: &mut BallState,
        gravity: Vec3,
        dt: f32,
        start: &[BodyPose],
        end: &[BodyPose],
    ) -> StepReport {
        self.step_motion(
            ball,
            gravity,
            dt,
            &mut LinearMotion {
                start,
                end,
                duration: dt,
                elapsed: 0.,
            },
        )
    }
    /// Gravity is integrated once. Each swept contact advances the motion clock;
    /// the next sweep samples actual path poses rather than interpolating a frame.
    pub fn step_motion(
        &self,
        ball: &mut BallState,
        gravity: Vec3,
        dt: f32,
        motion: &mut impl BodyMotion,
    ) -> StepReport {
        let mut report = StepReport::default();
        if dt <= 0.0 || !dt.is_finite() {
            return report;
        }
        ball.velocity += gravity * dt;
        let mut remaining = dt;
        for iteration in 0..16 {
            if remaining <= 0.0 {
                return report;
            }
            if iteration == 15 {
                report.exhausted = true;
                ball.advance(remaining);
                motion.advance(remaining);
                return report;
            }
            let limit = motion.boundary(remaining);
            let start = motion.poses(0.);
            // The executable samples a body chord at the current best sweep bound.
            // Within a body all brush tests share this velocity, including smooth paths.
            let mut nearest: Option<(Hit, usize, Vec3, Vec3, Vec3, bool)> = None;
            let mut body_cache = None;
            for collider in &self.colliders {
                let bound = nearest.map_or(limit, |h| h.0.time);
                let (a, lv, surface, rotating) = match body_cache {
                    Some((body, a, lv, surface, rotating)) if body == collider.body => {
                        (a, lv, surface, rotating)
                    }
                    _ => {
                        let a = start.get(collider.body).copied().unwrap_or_default();
                        let b = motion
                            .poses(bound)
                            .get(collider.body)
                            .copied()
                            .unwrap_or_default();
                        let surface = (b.translation - a.translation) / bound;
                        let rotating = motion.rotation_active(collider.body, bound);
                        let lv = if rotating {
                            let relative = ball.position - a.translation;
                            let end_relative = ball.velocity * bound + relative - surface * bound;
                            (rotate_vector(b.rotation.conjugate(), end_relative)
                                - rotate_vector(a.rotation.conjugate(), relative))
                                / bound
                        } else {
                            ball.velocity - surface
                        };
                        body_cache = Some((collider.body, a, lv, surface, rotating));
                        (a, lv, surface, rotating)
                    }
                };
                let lp = a.inverse_point(ball.position);
                let hit = if rotating {
                    collider.sweep(lp, lv, ball.radius, bound)
                } else {
                    collider.sweep_translated(
                        ball.position,
                        ball.velocity,
                        ball.radius,
                        bound,
                        a.translation,
                        surface,
                    )
                };
                if let Some(hit) = hit {
                    nearest = Some((hit, collider.body, lv, surface, a.translation, rotating));
                }
            }
            if let Some((
                hit,
                body,
                local_velocity,
                translation_velocity,
                start_translation,
                rotating,
            )) = nearest
            {
                let rotation = motion
                    .poses(hit.time)
                    .get(body)
                    .copied()
                    .unwrap_or_default()
                    .rotation;
                let point = if rotating {
                    rotate_vector(rotation, hit.point)
                        + start_translation
                        + translation_velocity * hit.time
                } else {
                    hit.point
                };
                let delta = ball.position + ball.velocity * hit.time - point;
                let normal = delta / delta.length();
                let surface_velocity = if rotating {
                    ball.velocity - rotate_vector(rotation, local_velocity)
                } else {
                    translation_velocity
                };
                let relative = ball.velocity - surface_velocity;
                ball.advance(hit.time);
                motion.advance(hit.time);
                remaining -= hit.time;
                let approach = relative.dot(normal);
                report.collisions += 1;
                report.max_impact = report.max_impact.max(approach.abs());
                ball.angular_velocity = delta.cross(relative) / (ball.radius * ball.radius);
                let impulse = (surface_velocity.dot(normal) - ball.velocity.dot(normal)) * 1.7;
                ball.velocity += normal * impulse;
                ball.position = point + normal * ball.radius;
            } else {
                ball.advance(limit);
                motion.advance(limit);
                remaining -= limit;
            }
        }
        report
    }
}
