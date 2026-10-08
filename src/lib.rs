pub mod camera;
pub mod content;
pub mod entities;
pub mod flow;
pub mod physics;
pub mod replay;
pub mod settings;
pub mod sol;
// SPDX-License-Identifier: GPL-3.0-or-later
use macroquad::prelude::{vec2, vec3, Vec2, Vec3};
use serde::Deserialize;

pub const STEP: f32 = 1.0 / 90.0;
#[derive(Clone, Deserialize)]
pub struct Triangle {
    pub p: [[f32; 3]; 3],
    pub n: [f32; 3],
    pub color: [f32; 3],
}
#[derive(Clone, Deserialize)]
pub struct Brush {
    pub planes: Vec<[f32; 4]>,
    pub lo: [f32; 3],
    pub hi: [f32; 3],
}
#[derive(Clone, Deserialize)]
pub struct Course {
    pub mesh: Vec<Triangle>,
    pub brushes: Vec<Brush>,
    pub start: [f32; 3],
    pub radius: f32,
    pub coins: Vec<[f32; 3]>,
    pub goal: [f32; 4],
    pub required_coins: usize,
    pub time_limit: f32,
}
impl Course {
    pub fn reference() -> Self {
        serde_json::from_str(include_str!("../fixtures/course.json"))
            .expect("checked course fixture")
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Playing,
    Won,
    Fell,
    TimedOut,
}
pub struct Game {
    pub course: Course,
    pub position: Vec3,
    pub velocity: Vec3,
    pub tilt: Vec2,
    pub collected: Vec<bool>,
    pub time: f32,
    pub outcome: Outcome,
    pub distance: f32,
}
impl Game {
    pub fn new(course: Course) -> Self {
        let position = Vec3::from_array(course.start);
        let count = course.coins.len();
        let time = course.time_limit;
        Self {
            course,
            position,
            velocity: Vec3::ZERO,
            tilt: Vec2::ZERO,
            collected: vec![false; count],
            time,
            outcome: Outcome::Playing,
            distance: 0.0,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::new(self.course.clone());
    }
    pub fn coins(&self) -> usize {
        self.collected.iter().filter(|x| **x).count()
    }
    pub fn goal_open(&self) -> bool {
        self.coins() >= self.course.required_coins
    }
    pub fn step(&mut self, input: Vec2, dt: f32) {
        if self.outcome != Outcome::Playing || dt <= 0.0 || !dt.is_finite() {
            return;
        }
        let dt = dt.min(STEP);
        self.tilt += (input.clamp(vec2(-1., -1.), vec2(1., 1.)) * 20. - self.tilt) * (4. * dt);
        let pitch = self.tilt.y.to_radians();
        let roll = self.tilt.x.to_radians();
        let gravity = vec3(
            9.8 * roll.sin() * pitch.cos(),
            -9.8 * pitch.cos() * roll.cos(),
            -9.8 * pitch.sin(),
        );
        let old = self.position;
        for _ in 0..2 {
            self.velocity += gravity * (dt * 0.5);
            self.position += self.velocity * (dt * 0.5);
            self.collide();
        }
        self.distance += (self.position - old).length();
        self.time = (self.time - dt).max(0.);
        for (i, c) in self.course.coins.iter().enumerate() {
            if !self.collected[i]
                && self.position.distance(Vec3::from_array(*c)) < self.course.radius + 0.15
            {
                self.collected[i] = true;
            }
        }
        let goal = vec3(
            self.course.goal[0],
            self.course.goal[1],
            self.course.goal[2],
        );
        if self.goal_open()
            && vec2(self.position.x - goal.x, self.position.z - goal.z).length()
                < self.course.goal[3] - self.course.radius
            && (self.position.y - goal.y).abs() < 1.
        {
            self.outcome = Outcome::Won;
        } else if self.position.y < -6. {
            self.outcome = Outcome::Fell;
        } else if self.time == 0. {
            self.outcome = Outcome::TimedOut;
        }
    }
    fn collide(&mut self) {
        let r = self.course.radius;
        for b in &self.course.brushes {
            let lo = Vec3::from_array(b.lo) - Vec3::splat(r);
            let hi = Vec3::from_array(b.hi) + Vec3::splat(r);
            if !self.position.cmpge(lo).all() || !self.position.cmple(hi).all() {
                continue;
            }
            let planes: Vec<(Vec3, f32)> = b
                .planes
                .iter()
                .map(|p| (vec3(p[0], p[1], p[2]), p[3]))
                .collect();
            let mut nearest = -f32::INFINITY;
            let mut normal = Vec3::ZERO;
            for &(n, d) in &planes {
                let distance = n.dot(self.position) - d;
                if distance > nearest {
                    nearest = distance;
                    normal = n;
                }
            }
            if nearest <= 0.0 {
                resolve(&mut self.position, &mut self.velocity, normal, r - nearest);
                continue;
            }
            // Find the closest point on the convex brush. Rounded sphere-edge contacts
            // must not collide with the square corner of an expanded plane intersection.
            let mut closest = None;
            let mut best = f32::INFINITY;
            for &(n, d) in &planes {
                consider(
                    self.position - n * (n.dot(self.position) - d),
                    self.position,
                    &planes,
                    &mut best,
                    &mut closest,
                );
            }
            if closest.is_none() {
                for i in 0..planes.len() {
                    for j in i + 1..planes.len() {
                        let (a, ad) = planes[i];
                        let (b, bd) = planes[j];
                        let ab = a.dot(b);
                        let det = 1. - ab * ab;
                        if det < 0.00001 {
                            continue;
                        }
                        let da = a.dot(self.position) - ad;
                        let db = b.dot(self.position) - bd;
                        consider(
                            self.position - a * ((da - ab * db) / det) - b * ((db - ab * da) / det),
                            self.position,
                            &planes,
                            &mut best,
                            &mut closest,
                        );
                    }
                }
            }
            if closest.is_none() {
                for i in 0..planes.len() {
                    for j in i + 1..planes.len() {
                        for k in j + 1..planes.len() {
                            let (a, ad) = planes[i];
                            let (b, bd) = planes[j];
                            let (c, cd) = planes[k];
                            let det = a.dot(b.cross(c));
                            if det.abs() < 0.00001 {
                                continue;
                            }
                            consider(
                                (b.cross(c) * ad + c.cross(a) * bd + a.cross(b) * cd) / det,
                                self.position,
                                &planes,
                                &mut best,
                                &mut closest,
                            );
                        }
                    }
                }
            }
            if let Some(point) = closest {
                let delta = self.position - point;
                let distance = delta.length();
                if distance < r && distance > 0.000001 {
                    resolve(
                        &mut self.position,
                        &mut self.velocity,
                        delta / distance,
                        r - distance,
                    );
                }
            }
        }
    }
}
fn resolve(position: &mut Vec3, velocity: &mut Vec3, normal: Vec3, depth: f32) {
    *position += normal * (depth + 0.00001);
    let approach = velocity.dot(normal);
    if approach < 0. {
        *velocity -= normal * (1.7 * approach);
    }
}
fn consider(
    p: Vec3,
    position: Vec3,
    planes: &[(Vec3, f32)],
    best: &mut f32,
    closest: &mut Option<Vec3>,
) {
    if planes.iter().all(|&(n, d)| n.dot(p) - d < 0.00005) {
        let distance = position.distance_squared(p);
        if distance < *best {
            *best = distance;
            *closest = Some(p);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rounded_edge_and_vertex_contacts() {
        let mut course = Course::reference();
        course.brushes = vec![Brush {
            planes: vec![
                [1., 0., 0., 1.],
                [-1., 0., 0., 0.],
                [0., 1., 0., 1.],
                [0., -1., 0., 0.],
                [0., 0., 1., 1.],
                [0., 0., -1., 0.],
            ],
            lo: [0.; 3],
            hi: [1.; 3],
        }];
        let mut game = Game::new(course);
        game.position = vec3(1.24, 1.24, 0.5);
        let clear = game.position;
        game.collide();
        assert_eq!(
            game.position, clear,
            "expanded square corners must not create a contact"
        );
        game.position = vec3(1.1, 1.1, 0.5);
        game.velocity = vec3(-1., -1., 0.);
        game.collide();
        assert!((game.position.distance(vec3(1., 1., 0.5)) - 0.25).abs() < 0.001);
        assert!(game.velocity.x > 0. && game.velocity.y > 0.);
        game.position = vec3(1.1, 1.1, 1.1);
        game.velocity = vec3(-1., -1., -1.);
        game.collide();
        assert!((game.position.distance(Vec3::ONE) - 0.25).abs() < 0.001);
        assert!(game.velocity.min_element() > 0.);
    }
    #[test]
    fn bounded_course_rules() {
        let c = Course::reference();
        assert_eq!(c.coins.len(), 18);
        assert_eq!(c.required_coins, 10);
        let mut g = Game::new(c);
        for _ in 0..900 {
            g.step(Vec2::ZERO, STEP);
        }
        assert!((g.position.x - 1.).abs() < 0.001);
        assert!((g.position.z + 1.).abs() < 0.001);
        assert!((g.position.y - 0.25).abs() < 0.002);
        assert!(!g.goal_open());
        for i in 0..10 {
            g.position = Vec3::from_array(g.course.coins[i]);
            g.velocity = Vec3::ZERO;
            g.step(Vec2::ZERO, STEP);
        }
        assert!(g.goal_open());
        g.position = vec3(g.course.goal[0], 0.251, g.course.goal[2]);
        g.step(Vec2::ZERO, STEP);
        assert_eq!(g.outcome, Outcome::Won);
        g.reset();
        assert_eq!(g.coins(), 0);
        assert_eq!(g.outcome, Outcome::Playing);
    }
    #[test]
    fn loss_pause_boundary_and_reset() {
        let mut g = Game::new(Course::reference());
        g.position = vec3(100., -7., 100.);
        g.step(Vec2::ZERO, STEP);
        assert_eq!(g.outcome, Outcome::Fell);
        let frozen = g.position;
        g.step(Vec2::ONE, STEP);
        assert_eq!(g.position, frozen);
        g.reset();
        g.time = STEP * 0.5;
        g.step(Vec2::ZERO, STEP);
        assert_eq!(g.outcome, Outcome::TimedOut);
    }
}
