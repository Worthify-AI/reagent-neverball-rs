// SPDX-License-Identifier: GPL-3.0-or-later
//! Level simulation derived from SOL records and generated decompilation of the reference executable.
use crate::physics::{BallState, BodyPose, Collider, PhysicsWorld, StepReport};
use crate::sol::Sol;
use crate::Outcome;
use macroquad::prelude::{vec2, vec3, Quat, Vec2, Vec3};
use std::collections::{BTreeSet, HashSet};
#[derive(Clone, Debug)]
pub struct Mover {
    pub path: usize,
    pub elapsed: f32,
    pub millis: i32,
}
#[derive(Clone, Debug)]
pub struct SwitchState {
    pub enabled: bool,
    pub entered: bool,
    pub elapsed: f32,
    pub millis: i32,
}
#[derive(Clone, Copy, Debug)]
pub struct JumpState {
    pub elapsed: f32,
    pub destination: Vec3,
    pub moved: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Item(usize),
    Coin(i32),
    Size(i32),
    Switch(usize, bool),
    JumpStarted,
    Teleported(Vec3),
    JumpFinished,
    GoalOpened,
    Won,
    Fell,
    TimedOut,
    Impact(f32),
}
#[derive(Clone, Debug)]
pub struct PathRuntime {
    pub enabled: Vec<bool>,
    pub movers: Vec<Mover>,
    pub body_movers: Vec<[Option<usize>; 2]>,
    fractional_second: f32,
}
fn quat(value: Option<[f32; 4]>) -> Quat {
    value
        .map(|q| Quat::from_xyzw(q[1], q[2], q[3], q[0]))
        .unwrap_or(Quat::IDENTITY)
}
fn duration(value: f32) -> f32 {
    (value * 1000.0).round().max(1.0) * 0.001
}
struct PathMotion<'a> {
    paths: &'a mut PathRuntime,
    sol: &'a Sol,
    switches: Option<(&'a mut [SwitchState], &'a mut Vec<Event>)>,
}
impl crate::physics::BodyMotion for PathMotion<'_> {
    fn boundary(&self, dt: f32) -> f32 {
        self.paths.boundary(self.sol, dt)
    }
    fn poses(&self, ahead: f32) -> Vec<BodyPose> {
        self.paths.poses(self.sol, ahead)
    }
    fn rotation_active(&self, body: usize, _ahead: f32) -> bool {
        let Some(index) = self.paths.body_movers.get(body).and_then(|p| p[1]) else {
            return false;
        };
        let p = &self.sol.paths[self.paths.movers[index].path];
        self.paths
            .poses(self.sol, 0.)
            .get(body)
            .is_some_and(|p| p.rotation != Quat::IDENTITY)
            || (self.paths.enabled[self.paths.movers[index].path]
                && (p.flags & 1 != 0
                    || self
                        .sol
                        .paths
                        .get(p.links[0].max(0) as usize)
                        .is_some_and(|p| p.flags & 1 != 0)))
    }
    fn advance(&mut self, dt: f32) {
        let millis = self.paths.advance_ticks(self.sol, dt);
        // FUN2ffd0 advances switch clocks with the same fractional-millisecond
        // accumulator as movers, after movers and before the next contact sweep.
        if let Some((states, events)) = self.switches.as_mut() {
            for (i, state) in states.iter_mut().enumerate() {
                let switch = &self.sol.switches[i];
                let end = (switch.time * 1000.).round() as i32;
                if state.millis < end {
                    state.millis += millis;
                    state.elapsed += dt;
                    if state.millis >= end {
                        state.enabled = switch.words[0] != 0;
                        if switch.path >= 0 {
                            self.paths
                                .set_chain(self.sol, switch.path as usize, state.enabled);
                        }
                        events.push(Event::Switch(i, state.enabled));
                    }
                }
            }
        }
    }
}
impl PathRuntime {
    pub fn step_ball(
        &mut self,
        sol: &Sol,
        world: &PhysicsWorld,
        ball: &mut BallState,
        gravity: Vec3,
        dt: f32,
    ) -> StepReport {
        world.step_motion(
            ball,
            gravity,
            dt,
            &mut PathMotion {
                paths: self,
                sol,
                switches: None,
            },
        )
    }

    pub fn new(sol: &Sol) -> Self {
        let enabled = sol.paths.iter().map(|p| p.links[1] != 0).collect();
        let mut movers = Vec::new();
        let mut body_movers = Vec::new();
        for body in &sol.bodies {
            let mut pair = [None, None];
            for axis in 0..2 {
                let path = if axis == 1 && body[axis] < 0 {
                    body[0]
                } else {
                    body[axis]
                };
                if path >= 0 && (path as usize) < sol.paths.len() {
                    let index = movers
                        .iter()
                        .position(|m: &Mover| m.path == path as usize)
                        .unwrap_or_else(|| {
                            movers.push(Mover {
                                path: path as usize,
                                elapsed: 0.,
                                millis: 0,
                            });
                            movers.len() - 1
                        });
                    pair[axis] = Some(index);
                }
            }
            body_movers.push(pair);
        }
        Self {
            enabled,
            movers,
            body_movers,
            fractional_second: 0.,
        }
    }
    pub fn set_chain(&mut self, sol: &Sol, path: usize, enabled: bool) {
        let mut path = path;
        let mut seen = HashSet::new();
        while path < sol.paths.len() && seen.insert(path) {
            self.enabled[path] = enabled;
            let next = sol.paths[path].links[0];
            if next < 0 {
                break;
            }
            path = next as usize;
        }
    }
    pub fn boundary(&self, sol: &Sol, mut limit: f32) -> f32 {
        for m in &self.movers {
            if !self.enabled[m.path] {
                continue;
            }
            let mut fraction = self.fractional_second + limit;
            let mut millis = 0;
            while fraction >= 0.001 {
                fraction -= 0.001;
                millis += 1;
            }
            let end = (sol.paths[m.path].duration * 1000.).round() as i32;
            if m.millis + millis > end {
                limit = (end - m.millis) as f32 * 0.001;
            }
        }
        limit
    }
    fn sample(&self, sol: &Sol, index: Option<usize>, ahead: f32) -> (Vec3, Quat) {
        let Some(index) = index else {
            return (Vec3::ZERO, Quat::IDENTITY);
        };
        let m = &self.movers[index];
        let p = &sol.paths[m.path];
        let next = sol.paths.get(p.links[0].max(0) as usize).unwrap_or(p);
        let elapsed = m.elapsed + if self.enabled[m.path] { ahead } else { 0. };
        let mut t = elapsed / duration(p.duration);
        if p.links[2] != 0 {
            t = 3.0 * t * t - (t + t) * t * t;
        }
        (
            (Vec3::from_array(next.position) - Vec3::from_array(p.position)) * t
                + Vec3::from_array(p.position),
            crate::physics::path_slerp(quat(p.rotation), quat(next.rotation), t),
        )
    }
    pub fn poses(&self, sol: &Sol, ahead: f32) -> Vec<BodyPose> {
        self.body_movers
            .iter()
            .map(|pair| BodyPose {
                translation: self.sample(sol, pair[0], ahead).0,
                rotation: self.sample(sol, pair[1], ahead).1,
            })
            .collect()
    }
    pub fn advance(&mut self, sol: &Sol, dt: f32) {
        self.advance_ticks(sol, dt);
    }
    fn advance_ticks(&mut self, sol: &Sol, dt: f32) -> i32 {
        self.fractional_second += dt;
        let mut millis = 0;
        while self.fractional_second >= 0.001 {
            self.fractional_second -= 0.001;
            millis += 1;
        }
        for m in &mut self.movers {
            if !self.enabled[m.path] {
                continue;
            }
            m.elapsed += dt;
            m.millis += millis;
            let p = &sol.paths[m.path];
            if m.millis >= (p.duration * 1000.).round() as i32 {
                m.elapsed = 0.;
                m.millis = 0;
                if p.links[0] >= 0 && (p.links[0] as usize) < sol.paths.len() {
                    m.path = p.links[0] as usize;
                }
            }
        }
        millis
    }
}
fn index_range(sol: &Sol, start: i32, count: i32) -> impl Iterator<Item = usize> + '_ {
    let a = start.max(0) as usize;
    let b = a
        .saturating_add(count.max(0) as usize)
        .min(sol.indices.len());
    sol.indices
        .get(a..b)
        .unwrap_or(&[])
        .iter()
        .copied()
        .filter(|&i| i >= 0)
        .map(|i| i as usize)
}
pub fn collision_world(sol: &Sol) -> PhysicsWorld {
    let mut colliders = Vec::new();
    for (body_index, body) in sol.bodies.iter().enumerate() {
        let mut lump_ids = BTreeSet::new();
        let mut nodes = vec![body[2]];
        let mut seen = HashSet::new();
        while let Some(n) = nodes.pop() {
            if n < 0 || !seen.insert(n) {
                continue;
            }
            if let Some(node) = sol.nodes.get(n as usize) {
                for l in node[3].max(0)..node[3].saturating_add(node[4]) {
                    lump_ids.insert(l as usize);
                }
                nodes.extend([node[1], node[2]]);
            }
        }
        // The body's direct lump interval includes the same BSP-owned brushes.
        for l in body[3].max(0)..body[3].saturating_add(body[4]) {
            lump_ids.insert(l as usize);
        }
        for lump_index in lump_ids {
            let Some(l) = sol.lumps.get(lump_index) else {
                continue;
            };
            if l[0] & 1 != 0 {
                continue;
            }
            let planes = index_range(sol, l[7], l[8])
                .filter_map(|i| sol.planes.get(i))
                .map(|p| (vec3(p[0], p[1], p[2]), p[3]))
                .collect();
            let vertices = index_range(sol, l[1], l[2])
                .filter_map(|i| sol.vertices.get(i))
                .map(|&v| Vec3::from_array(v))
                .collect();
            let edges = index_range(sol, l[3], l[4])
                .filter_map(|i| sol.edges.get(i))
                .filter_map(|e| {
                    Some((
                        Vec3::from_array(*sol.vertices.get(e[0] as usize)?),
                        Vec3::from_array(*sol.vertices.get(e[1] as usize)?),
                    ))
                })
                .collect();
            colliders.push(Collider::new(body_index, planes, vertices, edges));
        }
    }
    PhysicsWorld { colliders }
}
/// Gravity rotation follows binary 0x19e90/0xfb80: full-angle double sin/cos,
/// float matrix entries, then matrix multiplication. Quaternion shortcuts round differently.
pub fn tilted_gravity(tilt: Vec2, right: Vec3, back: Vec3, vertical: f32) -> Vec3 {
    fn matrix(axis: Vec3, degrees: f32) -> [[f32; 3]; 3] {
        let axis = axis / axis.length();
        let a = axis.to_array();
        let angle = (std::f32::consts::PI * degrees) / 180.;
        let cosine = (angle as f64).cos() as f32;
        let sine = (angle as f64).sin() as f32;
        let mut m = [[0.; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                let product = a[i] * a[j];
                m[i][j] = ((if i == j { 1. } else { 0. }) - product) * cosine + product;
            }
        }
        m[0][1] -= a[2] * sine;
        m[1][0] += a[2] * sine;
        m[0][2] += a[1] * sine;
        m[2][0] -= a[1] * sine;
        m[1][2] -= a[0] * sine;
        m[2][1] += a[0] * sine;
        m
    }
    let a = matrix(back, tilt.x);
    let b = matrix(right, tilt.y);
    Vec3::from_array(std::array::from_fn(|i| {
        ((a[i][0] * b[0][1] + a[i][1] * b[1][1]) + a[i][2] * b[2][1]) * vertical
    }))
}
pub struct FullGame {
    pub sol: Sol,
    pub world: PhysicsWorld,
    pub ball: BallState,
    pub paths: PathRuntime,
    pub body_poses: Vec<BodyPose>,
    pub tilt: Vec2,
    pub response_seconds: f32,
    pub time: f32,
    pub elapsed: f32,
    pub time_limit: f32,
    pub required_coins: i32,
    pub coins: i32,
    pub collected: Vec<bool>,
    pub switches: Vec<SwitchState>,
    pub outcome: Outcome,
    pub events: Vec<Event>,
    pub base_radius: f32,
    pub size: i32,
    pub jump: Option<JumpState>,
    pub jump_enabled: bool,
    pub distance: f32,
    radius_from: f32,
    radius_target: f32,
    radius_elapsed: f32,
}
impl FullGame {
    pub fn new(sol: Sol) -> Self {
        let start = sol.balls.first().copied().unwrap_or([0., 1., 0., 0.25]);
        let ball = BallState::new(vec3(start[0], start[1], start[2]), start[3]);
        let world = collision_world(&sol);
        let paths = PathRuntime::new(&sol);
        let body_poses = paths.poses(&sol, 0.);
        let collected = vec![false; sol.items.len()];
        let switches = sol
            .switches
            .iter()
            .map(|s| SwitchState {
                enabled: s.words[0] != 0,
                entered: false,
                elapsed: duration(s.time),
                millis: (s.time * 1000.).round() as i32,
            })
            .collect();
        let time_limit = sol
            .metadata
            .get("time")
            .and_then(|s| s.parse::<f32>().ok())
            .unwrap_or(0.)
            / 100.;
        let required_coins = sol
            .metadata
            .get("goal")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        Self {
            sol,
            world,
            ball,
            paths,
            body_poses,
            tilt: Vec2::ZERO,
            response_seconds: 0.25,
            time: time_limit,
            elapsed: 0.,
            time_limit,
            required_coins,
            coins: 0,
            collected,
            switches,
            outcome: Outcome::Playing,
            events: Vec::new(),
            base_radius: start[3],
            size: 0,
            jump: None,
            jump_enabled: true,
            distance: 0.,
            radius_from: start[3],
            radius_target: start[3],
            radius_elapsed: 0.5,
        }
    }
    pub fn reset(&mut self) {
        *self = Self::new(self.sol.clone());
    }
    pub fn goal_open(&self) -> bool {
        self.coins >= self.required_coins
    }
    pub fn position(&self) -> Vec3 {
        self.ball.position
    }
    pub fn radius(&self) -> f32 {
        self.ball.radius
    }
    /// Input is normalized mouse/key tilt. Camera basis is the previous camera frame.
    pub fn step(&mut self, input: Vec2, right: Vec3, back: Vec3, dt: f32) -> StepReport {
        if self.outcome != Outcome::Playing {
            self.events.clear();
            return StepReport::default();
        }
        self.step_mode(input, right, back, dt, true)
    }
    /// Terminal presentation follows binary dispatcher 0x1c2a0: goal reverses gravity,
    /// fall continues gravity, and timeout stops the simulation. Scoring stays frozen.
    pub fn step_result(&mut self, input: Vec2, right: Vec3, back: Vec3, dt: f32) -> StepReport {
        if !matches!(self.outcome, Outcome::Won | Outcome::Fell) {
            self.events.clear();
            return StepReport::default();
        }
        self.step_mode(input, right, back, dt, false)
    }
    fn step_mode(
        &mut self,
        input: Vec2,
        right: Vec3,
        back: Vec3,
        dt: f32,
        playing: bool,
    ) -> StepReport {
        self.events.clear();
        let mut report = StepReport::default();
        if !dt.is_finite() || dt <= 0. {
            return report;
        }
        let dt = dt.min(0.1);
        self.tilt += (input.clamp(vec2(-1., -1.), vec2(1., 1.)) * 20. - self.tilt)
            * (dt / self.response_seconds.max(dt));
        let gravity = tilted_gravity(
            self.tilt,
            right,
            back,
            if self.outcome == Outcome::Won {
                9.8
            } else {
                -9.8
            },
        );
        let old = self.ball.position;
        if self.radius_elapsed < 0.5 {
            self.radius_elapsed = (self.radius_elapsed + dt).min(0.5);
            let r = self.radius_from
                + (self.radius_target - self.radius_from) * (self.radius_elapsed / 0.5);
            self.ball.position.y += r - self.ball.radius;
            self.ball.radius = r;
        }
        if let Some(mut jump) = self.jump {
            jump.elapsed += dt;
            if jump.elapsed >= 0.5 {
                self.ball.position = jump.destination;
                if !jump.moved {
                    self.events.push(Event::Teleported(jump.destination));
                    jump.moved = true;
                }
            }
            if jump.elapsed >= 1. {
                self.jump = None;
                self.events.push(Event::JumpFinished);
            } else {
                self.jump = Some(jump);
            }
        } else {
            let previous_velocity = self.ball.velocity;
            self.ball.velocity += gravity * dt;
            report = self.world.step_motion(
                &mut self.ball,
                Vec3::ZERO,
                dt,
                &mut PathMotion {
                    paths: &mut self.paths,
                    sol: &self.sol,
                    switches: Some((&mut self.switches, &mut self.events)),
                },
            );
            self.ball.update_inner(previous_velocity, gravity, dt);
            self.body_poses = self.paths.poses(&self.sol, 0.);
            if report.max_impact > 0.5 {
                self.events.push(Event::Impact(report.max_impact));
            }
        }
        self.distance += self.ball.position.distance(old);
        if playing {
            self.elapsed += dt;
            self.time = if self.time_limit > 0. {
                (self.time - dt).max(0.)
            } else {
                self.elapsed
            };
            self.collect_items();
        }
        self.enter_switches();
        self.enter_jump();
        if playing {
            self.check_outcome();
        }
        report
    }
    pub fn collect_items(&mut self) {
        let was_open = self.goal_open();
        // Original consumes at most one item per simulation update.
        for (i, item) in self.sol.items.iter().enumerate() {
            if self.collected[i]
                || item.kind == 0
                || self.ball.position.distance(Vec3::from_array(item.position))
                    >= self.ball.radius + 0.15
            {
                continue;
            }
            self.collected[i] = true;
            self.events.push(Event::Item(i));
            match item.kind {
                1 => {
                    self.coins += item.value;
                    self.events.push(Event::Coin(item.value));
                }
                2 | 3 => {
                    let next = (self.size + if item.kind == 2 { 1 } else { -1 }).clamp(-1, 1);
                    self.size = next;
                    self.radius_from = self.ball.radius;
                    self.radius_target = self.base_radius
                        * match next {
                            -1 => 0.5,
                            1 => 1.5,
                            _ => 1.,
                        };
                    self.radius_elapsed = 0.;
                    self.events.push(Event::Size(next));
                }
                _ => {}
            }
            break;
        }
        if !was_open && self.goal_open() {
            self.events.push(Event::GoalOpened);
        }
    }
    pub fn enter_switches(&mut self) {
        let p = self.ball.position;
        let r = self.ball.radius;
        for (i, state) in self.switches.iter_mut().enumerate() {
            let s = &self.sol.switches[i];
            if s.time > 0. && state.enabled != (s.words[0] != 0) {
                continue;
            }
            let horizontal = vec2(p.x - s.position[0], p.z - s.position[2]).length();
            if horizontal > s.radius + r || p.y <= s.position[1] || p.y >= s.position[1] + 1. {
                state.entered = false;
                continue;
            }
            if state.entered || horizontal + r > s.radius {
                continue;
            }
            if s.time == 0. {
                state.entered = true;
            }
            state.enabled = !state.enabled;
            state.elapsed = 0.;
            state.millis = 0;
            if s.path >= 0 {
                self.paths
                    .set_chain(&self.sol, s.path as usize, state.enabled);
            }
            self.events.push(Event::Switch(i, state.enabled));
        }
    }
    pub fn enter_jump(&mut self) {
        if self.jump.is_some() {
            return;
        }
        let p = self.ball.position;
        let r = self.ball.radius;
        let mut touching = false;
        for j in &self.sol.jumps {
            let distance = vec2(p.x - j[0], p.z - j[2]).length();
            if distance <= j[6] && p.y > j[1] && p.y < j[1] + 1. {
                touching = true;
                if self.jump_enabled && distance + r <= j[6] {
                    self.jump = Some(JumpState {
                        elapsed: 0.,
                        destination: vec3(j[3], j[4], j[5]) + p - vec3(j[0], j[1], j[2]),
                        moved: false,
                    });
                    self.jump_enabled = false;
                    self.events.push(Event::JumpStarted);
                    break;
                }
            }
        }
        if !touching {
            self.jump_enabled = true;
        }
    }
    pub fn check_outcome(&mut self) {
        let p = self.ball.position;
        if self.goal_open()
            && self.sol.goals.iter().any(|g| {
                vec2(p.x - g[0], p.z - g[2]).length() + self.ball.radius < g[3]
                    && p.y > g[1]
                    && p.y < g[1] + 1.5
            })
        {
            self.outcome = Outcome::Won;
            self.events.push(Event::Won);
        } else if self.time_limit > 0. && self.time <= 0. {
            self.outcome = Outcome::TimedOut;
            self.events.push(Event::TimedOut);
        } else if self.sol.vertices.first().is_none_or(|v| p.y < v[1]) {
            self.outcome = Outcome::Fell;
            self.events.push(Event::Fell);
        }
    }
}
