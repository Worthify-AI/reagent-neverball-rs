// SPDX-License-Identifier: GPL-3.0-or-later
use macroquad::prelude::{vec3, Quat, Vec2, Vec3};
use reagent_neverball_rs::{
    entities::{FullGame, PathRuntime},
    physics::{BallState, BodyPose, Collider, PhysicsWorld},
    sol::{Item, Path, Sol, Switch},
    Outcome,
};
fn cube() -> Collider {
    let vertices = (0..8)
        .map(|i| vec3((i >> 2) as f32, ((i >> 1) & 1) as f32, (i & 1) as f32))
        .collect::<Vec<_>>();
    let mut edges = Vec::new();
    for i in 0usize..8 {
        for j in i + 1..8 {
            if (i ^ j).count_ones() == 1 {
                edges.push((vertices[i], vertices[j]));
            }
        }
    }
    Collider::new(
        0,
        vec![
            (Vec3::X, 1.),
            (-Vec3::X, 0.),
            (Vec3::Y, 1.),
            (-Vec3::Y, 0.),
            (Vec3::Z, 1.),
            (-Vec3::Z, 0.),
        ],
        vertices,
        edges,
    )
}
fn empty() -> Sol {
    Sol {
        vertices: vec![[0., -100., 0.]],
        balls: vec![[0., 0.25, 0., 0.25]],
        ..Default::default()
    }
}
#[test]
fn swept_face_edge_vertex_and_square_miss() {
    let c = cube();
    let face = c
        .sweep(vec3(0.5, 3., 0.5), vec3(0., -4., 0.), 0.25, 1.)
        .unwrap();
    assert!((face.time - 0.4375).abs() < 1e-6);
    assert_eq!(face.point, vec3(0.5, 1., 0.5));
    let edge = c
        .sweep(vec3(2., 2., 0.5), vec3(-4., -4., 0.), 0.25, 1.)
        .unwrap();
    assert!((edge.time - (1. - 0.25 / 2f32.sqrt()) / 4.).abs() < 1e-6);
    let vertex = c
        .sweep(Vec3::splat(2.), Vec3::splat(-4.), 0.25, 1.)
        .unwrap();
    assert!((vertex.time - (1. - 0.25 / 3f32.sqrt()) / 4.).abs() < 1e-6);
    assert!(c
        .sweep(vec3(1.24, 1.24, -1.), vec3(0., 0., 4.), 0.25, 1.)
        .is_none());
}
#[test]
fn high_speed_sphere_does_not_tunnel() {
    let world = PhysicsWorld {
        colliders: vec![cube()],
    };
    let mut b = BallState::new(vec3(0.5, 100., 0.5), 0.25);
    b.velocity = vec3(0., -20000., 0.);
    let report = world.step(&mut b, Vec3::ZERO, 0.02, &[], &[]);
    assert_eq!(report.collisions, 1);
    assert!(b.position.y > 1.25);
    assert!((b.velocity.y - 14000.).abs() < 0.01);
}
#[test]
fn moving_platform_relative_impact() {
    let world = PhysicsWorld {
        colliders: vec![cube()],
    };
    let mut b = BallState::new(vec3(0.5, 2., 0.5), 0.25);
    let a = BodyPose::default();
    let z = BodyPose {
        translation: Vec3::Y,
        rotation: Quat::IDENTITY,
    };
    let r = world.step(&mut b, Vec3::ZERO, 1., &[a], &[z]);
    assert_eq!(r.collisions, 1);
    assert!((b.velocity.y - 1.7).abs() < 1e-5);
    assert!(b.position.y > 2.25);
}
#[test]
fn rolling_orientation_tracks_tangential_contact() {
    let world = PhysicsWorld {
        colliders: vec![cube()],
    };
    let mut b = BallState::new(vec3(0.5, 1.25, 0.5), 0.25);
    b.velocity = vec3(1., -1., 0.);
    world.step(&mut b, Vec3::ZERO, 0.01, &[], &[]);
    assert!((b.angular_velocity.z + 4.).abs() < 1e-5);
    assert!(b.orientation.z < 0.);
}
fn path(position: [f32; 3], next: i32, enabled: i32, smooth: i32) -> Path {
    Path {
        position,
        duration: 1.,
        links: [next, enabled, smooth],
        ..Default::default()
    }
}
#[test]
fn path_linear_smooth_rotational_and_boundary() {
    let mut sol = empty();
    sol.paths = vec![path([0.; 3], 1, 1, 0), path([2., 0., 0.], 0, 1, 0)];
    sol.paths[1].rotation = Some([0., 0., 1., 0.]);
    sol.bodies = vec![[0, 0, -1, 0, 0, 0, 0]];
    let mut runtime = PathRuntime::new(&sol);
    let p = runtime.poses(&sol, 0.25)[0];
    assert!((p.translation.x - 0.5).abs() < 1e-6);
    assert!((p.rotation * Vec3::X).distance(vec3(2f32.sqrt() / 2., 0., -2f32.sqrt() / 2.)) < 1e-5);
    sol.paths[0].links[2] = 1;
    assert!((runtime.poses(&sol, 0.25)[0].translation.x - 0.3125).abs() < 1e-6);
    runtime.advance(&sol, 1.);
    assert_eq!(runtime.movers[0].path, 1);
    assert_eq!(runtime.poses(&sol, 0.)[0].translation, Vec3::X * 2.);
    runtime.set_chain(&sol, 0, false);
    assert_eq!(runtime.enabled, vec![false, false]);
    let frozen = runtime.poses(&sol, 0.)[0];
    runtime.advance(&sol, 5.);
    assert_eq!(runtime.poses(&sol, 0.)[0].translation, frozen.translation);
}
#[test]
fn switch_entry_hysteresis_and_timer_reversion() {
    let mut sol = empty();
    sol.paths = vec![path([0.; 3], 1, 0, 0), path([2., 0., 0.], 0, 0, 0)];
    sol.switches = vec![Switch {
        position: [0.; 3],
        radius: 1.,
        path: 0,
        time: 0.,
        ..Default::default()
    }];
    let mut g = FullGame::new(sol);
    g.enter_switches();
    assert!(g.paths.enabled.iter().all(|s| *s));
    g.enter_switches();
    assert!(g.switches[0].enabled);
    g.ball.position.x = 2.;
    g.enter_switches();
    g.ball.position.x = 0.;
    g.enter_switches();
    assert!(!g.switches[0].enabled);
    g.sol.switches[0].time = 0.03;
    g.enter_switches();
    assert!(!g.switches[0].enabled);
    g.ball.position.x = 2.;
    g.enter_switches();
    g.ball.position.x = 0.;
    g.enter_switches();
    assert!(g.switches[0].enabled);
    g.ball.position.x = 2.;
    g.step(Vec2::ZERO, Vec3::X, Vec3::Z, 0.04);
    assert!(!g.switches[0].enabled);
    assert!(g.paths.enabled.iter().all(|s| !*s));
}
#[test]
fn item_values_size_bounds_transition_and_floor_height() {
    let mut sol = empty();
    sol.items = vec![
        Item {
            position: [0., 0.25, 0.],
            kind: 1,
            value: 5,
        },
        Item {
            position: [0., 0.25, 0.],
            kind: 2,
            value: 1,
        },
    ];
    sol.metadata.insert("goal".into(), "5".into());
    let mut g = FullGame::new(sol);
    g.collect_items();
    assert_eq!(g.coins, 5);
    assert!(g.goal_open());
    assert_eq!(g.collected, vec![true, false]);
    g.collect_items();
    assert_eq!(g.size, 1);
    g.step(Vec2::ZERO, Vec3::X, Vec3::Z, 0.1);
    assert!((g.ball.radius - 0.275).abs() < 1e-6);
}
#[test]
fn jump_preserves_offset_and_velocity_and_rearms_after_exit() {
    let mut sol = empty();
    sol.jumps = vec![[0., 0., 0., 5., 3., 4., 1.], [5., 3., 4., 0., 0., 0., 1.]];
    let mut g = FullGame::new(sol);
    g.ball.position = vec3(0.1, 0.25, 0.2);
    g.ball.velocity = Vec3::X;
    g.enter_jump();
    assert!(g.jump.is_some());
    let destination = g.jump.unwrap().destination;
    assert_eq!(destination, vec3(5.1, 3.25, 4.2));
    for _ in 0..5 {
        g.step(Vec2::ZERO, Vec3::X, Vec3::Z, 0.1);
    }
    assert!(g.ball.position.distance(destination) < 1e-5);
    assert_eq!(g.ball.velocity.x, 1.);
    for _ in 0..5 {
        g.step(Vec2::ZERO, Vec3::X, Vec3::Z, 0.1);
    }
    assert!(g.jump.is_none());
    assert!(!g.jump_enabled);
    g.ball.position.x = 10.;
    g.enter_jump();
    assert!(g.jump_enabled);
}
#[test]
fn goal_full_containment_fall_time_and_reset() {
    let mut sol = empty();
    sol.goals = vec![[0., 0., 0., 1.]];
    sol.metadata.insert("time".into(), "100".into());
    let mut g = FullGame::new(sol);
    g.ball.position.x = 0.8;
    g.check_outcome();
    assert_eq!(g.outcome, Outcome::Playing);
    g.ball.position.x = 0.;
    g.check_outcome();
    assert_eq!(g.outcome, Outcome::Won);
    g.reset();
    g.ball.position = vec3(5., -101., 0.);
    g.check_outcome();
    assert_eq!(g.outcome, Outcome::Fell);
    g.reset();
    g.ball.position.x = 5.;
    g.time = 0.;
    g.check_outcome();
    assert_eq!(g.outcome, Outcome::TimedOut);
}
#[test]
fn compiled_all_levels_idle_smoke() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    let mut count = 0;
    let mut dynamic = 0;
    let mut switches = 0;
    let mut jumps = 0;
    let mut sizes = 0;
    for dir in std::fs::read_dir(&root).unwrap().flatten().filter(|d| {
        d.file_name().to_string_lossy().starts_with("map-") && d.file_name() != "map-back"
    }) {
        for file in std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .filter(|f| f.path().extension().is_some_and(|x| x == "sol"))
        {
            let sol = Sol::from_bytes(&std::fs::read(file.path()).unwrap()).unwrap();
            dynamic += usize::from(!sol.paths.is_empty());
            switches += usize::from(!sol.switches.is_empty());
            jumps += usize::from(!sol.jumps.is_empty());
            sizes += usize::from(sol.items.iter().any(|i| i.kind == 2 || i.kind == 3));
            let mut game = FullGame::new(sol);
            assert!(
                !game.world.colliders.is_empty(),
                "{}",
                file.path().display()
            );
            for _ in 0..180 {
                game.step(Vec2::ZERO, Vec3::X, Vec3::Z, 1. / 90.);
                assert!(game.ball.position.is_finite());
                assert!(game.ball.orientation.is_finite());
            }
            count += 1;
        }
    }
    assert_eq!(count, 183);
    assert!(dynamic > 20 && switches > 5 && jumps > 5 && sizes > 5);
    println!("loaded and stepped {count} levels; dynamic {dynamic}, switches {switches}, jumps {jumps}, sizes {sizes}");
}

#[test]
fn executable_differential_collision_paths_and_jumps() {
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/physics/component-results.json");
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let vector = |v: &serde_json::Value| {
        vec3(
            v[0].as_f64().unwrap() as f32,
            v[1].as_f64().unwrap() as f32,
            v[2].as_f64().unwrap() as f32,
        )
    };
    for case in data["collision"].as_array().unwrap() {
        let limit = case["limit"].as_f64().unwrap() as f32;
        let hit = cube().sweep(
            vector(&case["position"]),
            vector(&case["velocity"]),
            case["radius"].as_f64().unwrap() as f32,
            limit,
        );
        let expected = case["time"].as_f64().unwrap() as f32;
        if expected >= limit {
            assert!(hit.is_none());
        } else {
            let h = hit.unwrap();
            assert!((h.time - expected).abs() < 1e-6, "{}", case["name"]);
            assert!(
                h.point.distance(vector(&case["point"])) < 1e-5,
                "{}",
                case["name"]
            );
        }
    }
    let mut sol = empty();
    sol.paths = vec![path([0.; 3], 1, 1, 0), path([2., 0., 0.], 0, 1, 0)];
    sol.paths[1].rotation = Some([0., 0., 1., 0.]);
    sol.bodies = vec![[0, 0, -1, 0, 0, 0, 0]];
    for case in data["paths"].as_array().unwrap() {
        sol.paths[0].links[2] = case["smooth"].as_i64().unwrap() as i32;
        let runtime = PathRuntime::new(&sol);
        let p = runtime.poses(&sol, case["t"].as_f64().unwrap() as f32)[0];
        assert!(p.translation.distance(vector(&case["position"])) < 1e-6);
        // The direct executable fixture encodes rotation flags on both paths.
        sol.paths[0].flags = 1;
        sol.paths[1].flags = 1;
        let q = &case["rotation"];
        let expected = Quat::from_xyzw(
            q[1].as_f64().unwrap() as f32,
            q[2].as_f64().unwrap() as f32,
            q[3].as_f64().unwrap() as f32,
            q[0].as_f64().unwrap() as f32,
        );
        assert!(p.rotation.dot(expected).abs() > 0.999999);
    }
    let mut sol = empty();
    sol.jumps = vec![[0., 0., 0., 5., 3., 4., 1.]];
    for case in data["jumps"].as_array().unwrap() {
        let mut g = FullGame::new(sol.clone());
        g.ball.position = vector(&case["position"]);
        g.enter_jump();
        if case["status"] == 1 {
            assert!(
                g.jump
                    .unwrap()
                    .destination
                    .distance(vector(&case["destination"]))
                    < 1e-6
            );
        } else {
            assert!(g.jump.is_none());
        }
    }
}
#[test]
fn executable_core_trajectories() {
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/physics/component-results.json");
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let vector = |v: &serde_json::Value| {
        vec3(
            v[0].as_f64().unwrap() as f32,
            v[1].as_f64().unwrap() as f32,
            v[2].as_f64().unwrap() as f32,
        )
    };
    for case in data["core"].as_array().unwrap() {
        let mut sol = empty();
        sol.paths = vec![
            path([0.; 3], 1, 1, 0),
            path(vector(&case["target"]).to_array(), 0, 1, 0),
        ];
        // The direct executable fixture encodes rotation flags on both paths.
        sol.paths[0].flags = 1;
        sol.paths[1].flags = 1;
        let q = &case["rotation"];
        sol.paths[1].rotation = Some([
            q[0].as_f64().unwrap() as f32,
            q[1].as_f64().unwrap() as f32,
            q[2].as_f64().unwrap() as f32,
            q[3].as_f64().unwrap() as f32,
        ]);
        sol.bodies = vec![[0, 0, -1, 0, 0, 0, 0]];
        let mut paths = PathRuntime::new(&sol);
        let world = PhysicsWorld {
            colliders: vec![cube()],
        };
        let mut ball = BallState::new(vector(&case["start"]), 0.25);
        ball.velocity = vector(&case["velocity"]);
        let mut max_position = 0f32;
        let mut max_velocity = 0f32;
        for (i, frame) in case["frames"].as_array().unwrap().iter().enumerate() {
            let dt = 1. / 90.;
            paths.step_ball(&sol, &world, &mut ball, vector(&case["gravity"]), dt);
            let error = ball.position.distance(vector(&frame["position"]));
            max_position = max_position.max(error);
            max_velocity = max_velocity.max(ball.velocity.distance(vector(&frame["velocity"])));
            if error > 0.01 && i < 10 {
                println!(
                    "{} frame{i} error{error} reference{} rust{:?}",
                    case["name"], frame["position"], ball.position
                );
            }
        }
        println!(
            "{} max position {max_position} velocity {max_velocity}",
            case["name"]
        );
        assert!(max_velocity < 0.001);
        assert!(
            max_position < 0.0001,
            "{} position error {max_position}",
            case["name"]
        );
    }
}
#[test]
fn executable_item_radius_transitions() {
    let file = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/physics/component-results.json");
    let data: serde_json::Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
    let mut sol = empty();
    sol.vertices[0][1] = -1000.;
    sol.items = vec![Item::default()];
    let mut g = FullGame::new(sol);
    for case in data["entities"].as_array().unwrap() {
        g.sol.items[0] = Item {
            position: g.ball.position.to_array(),
            kind: case["kind"].as_i64().unwrap() as i32,
            value: 5,
        };
        g.collected[0] = false;
        g.collect_items();
        let mut intermediate = Vec::new();
        for step in 0..45 {
            g.step(Vec2::ZERO, Vec3::X, Vec3::Z, 1. / 90.);
            if [0, 8, 21, 44].contains(&step) {
                intermediate.push(g.ball.radius);
            }
        }
        assert_eq!(g.coins, case["coins"].as_i64().unwrap() as i32);
        assert_eq!(g.size, case["size"].as_i64().unwrap() as i32);
        assert!((g.ball.radius - case["radius"].as_f64().unwrap() as f32).abs() < 1e-6);
        for (i, r) in intermediate.iter().enumerate() {
            assert!((r - case["intermediate"][i].as_f64().unwrap() as f32).abs() < 1e-6);
        }
    }
}

#[test]
fn executable_replay_outer_and_inner_ball_orientation() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let sol_path = root.join("data/map-easy/easy.sol");
    for file in [
        root.join("fixtures/physics/recordings/orientation-straight.nbr"),
        root.join("fixtures/physics/recordings/orientation-turning.nbr"),
    ] {
        let mut game = FullGame::new(Sol::from_bytes(&std::fs::read(&sol_path).unwrap()).unwrap());
        let bytes = std::fs::read(&file).unwrap();
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
        let mut outer = [1., 0., 0., 0., 1., 0.];
        let mut inner = outer;
        let mut max_outer = 0f32;
        let mut max_inner = 0f32;
        let mut frames = 0;
        while at + 3 <= bytes.len() {
            let cmd = bytes[at];
            let n = u16::from_le_bytes(bytes[at + 1..at + 3].try_into().unwrap()) as usize;
            at += 3;
            let q = &bytes[at..at + n];
            at += n;
            let floats = || {
                q.chunks_exact(4)
                    .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
                    .collect::<Vec<_>>()
            };
            match cmd {
                5 => tilt.copy_from_slice(&floats()),
                23 => outer.copy_from_slice(&floats()),
                24 => inner.copy_from_slice(&floats()),
                32 => basis.copy_from_slice(&floats()),
                30 => dt += floats()[0],
                1 => {
                    if dt > 0. {
                        let right = Vec3::from_slice(&basis[..3]);
                        let back = Vec3::from_slice(&basis[3..]);
                        let rotation = Quat::from_axis_angle(back, tilt[1].to_radians())
                            * Quat::from_axis_angle(right, tilt[0].to_radians());
                        let gravity = rotation * vec3(0., -9.8, 0.);
                        let previous = game.ball.velocity;

                        game.paths
                            .step_ball(&game.sol, &game.world, &mut game.ball, gravity, dt);
                        game.ball.update_inner(previous, gravity, dt);
                        let error = |rotation: Quat, reference: [f32; 6]| {
                            (rotation * Vec3::X - Vec3::from_slice(&reference[..3]))
                                .length()
                                .max(
                                    (rotation * Vec3::Y - Vec3::from_slice(&reference[3..]))
                                        .length(),
                                )
                        };
                        max_outer = max_outer.max(error(game.ball.orientation, outer));
                        max_inner = max_inner.max(error(game.ball.inner_orientation, inner));
                        frames += 1;
                    }
                    dt = 0.;
                }
                _ => {}
            }
        }
        println!(
            "{}: {frames} frames outer basis {max_outer}, inner basis {max_inner}",
            file.display()
        );
        assert_eq!(frames, 1123);
        assert!(max_outer < 0.001);
        assert!(max_inner < 0.0001);
    }
}

#[test]
fn jump_holds_physics_and_paths_until_teleport_finishes() {
    let mut sol = empty();
    sol.paths.push(Path {
        position: [0., 0., 0.],
        duration: 10.,
        links: [0, 1, 0],
        flags: 0,
        rotation: None,
    });
    sol.bodies.push([0, -1, -1, 0, 0, 0, 0]);
    let mut game = FullGame::new(sol);
    game.ball.velocity = vec3(2., 3., 4.);
    let initial = game.ball.position;
    let destination = vec3(10., 2., 3.);
    game.jump = Some(reagent_neverball_rs::entities::JumpState {
        elapsed: 0.,
        destination,
        moved: false,
    });
    for frame in 0..10 {
        game.step(Vec2::ZERO, Vec3::X, Vec3::Z, 0.1);
        assert_eq!(game.ball.velocity, vec3(2., 3., 4.));
        assert_eq!(game.paths.movers[0].elapsed, 0.);
        assert_eq!(
            game.ball.position,
            if frame < 4 { initial } else { destination }
        );
    }
    assert!(game.jump.is_none());
    game.step(Vec2::ZERO, Vec3::X, Vec3::Z, 0.1);
    assert!(game.paths.movers[0].elapsed > 0.);
    assert_ne!(game.ball.position, destination);
}
#[test]
fn result_physics_reverses_goal_gravity_and_preserves_scoring() {
    for outcome in [Outcome::Won, Outcome::Fell, Outcome::TimedOut] {
        let mut game = FullGame::new(empty());
        game.outcome = outcome;
        game.elapsed = 12.;
        game.time = 45.;
        game.coins = 7;
        let before = game.ball.position;
        game.step_result(Vec2::ZERO, Vec3::X, Vec3::Z, 0.1);
        assert_eq!(
            (game.elapsed, game.time, game.coins, game.outcome),
            (12., 45., 7, outcome)
        );
        match outcome {
            Outcome::Won => assert!(game.ball.position.y > before.y),
            Outcome::Fell => assert!(game.ball.position.y < before.y),
            Outcome::TimedOut => assert_eq!(game.ball.position, before),
            _ => unreachable!(),
        }
    }
}

#[test]
fn actual_raw_key_seven_course_holdouts() {
    // Actual SDL ledgers from isolated reference runs; these are input-only runs.
    // Jump frames lack mover-dt commands, so the comparator uses the fixed90Hz clock.
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for (folder, name) in [
        ("easy", "roundcoins"),
        ("medium", "easytele"),
        ("medium", "learngrow"),
        ("medium", "locks"),
        ("easy", "curved"),
        ("easy", "thwomp2"),
        ("fwp", "adventure"),
    ] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_physics-course-holdout"))
            .arg(root.join(format!("data/map-{folder}/{name}.sol")))
            .arg(root.join(format!("fixtures/physics/recordings/{name}.nbr")))
            .arg(root.join(format!("fixtures/physics/recordings/{name}-ledger.csv")))
            .output()
            .expect("run raw-key comparator");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let data: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(
            data["max_position_error"].as_f64().unwrap() < 0.00001,
            "{name}: {data}"
        );
        assert_eq!(data["coin_mismatch_batches"], 0);
        assert_eq!(data["max_radius_error"], 0.0);
        if name == "easytele" {
            assert_eq!(data["reference_command_counts"]["10"], 1);
        } else if name == "adventure" {
            assert_eq!(data["reference_command_counts"]["19"], 46);
        } else if name == "roundcoins" {
            assert_eq!(data["reference_command_counts"]["16"], 4);
            assert_eq!(data["rust_coins"], 20);
        }
    }
}

#[test]
fn executable_fractional_millisecond_switch_expiry() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/physics");
    let reference: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("switch-clock-results.json")).unwrap())
            .unwrap();
    for case in reference["cases"].as_array().unwrap() {
        let mut sol = empty();
        sol.paths = vec![path([0.; 3], 1, 0, 0), path([1., 0., 0.], 0, 0, 0)];
        sol.switches = vec![Switch {
            position: [0.; 3],
            radius: 1.,
            path: 0,
            time: 0.03,
            ..Default::default()
        }];
        let mut game = FullGame::new(sol);
        game.ball.position.x = 2.;
        game.step(
            Vec2::ZERO,
            Vec3::X,
            Vec3::Z,
            case["prime"].as_f64().unwrap() as f32,
        );
        game.ball.position.x = 0.;
        game.enter_switches();
        game.ball.position.x = 2.;
        for sample in case["frames"].as_array().unwrap() {
            game.step(
                Vec2::ZERO,
                Vec3::X,
                Vec3::Z,
                sample["dt"].as_f64().unwrap() as f32,
            );
            let state = &game.switches[0];
            assert_eq!(state.millis, sample["millis"].as_i64().unwrap() as i32);
            assert_eq!(state.enabled, sample["enabled"].as_i64().unwrap() != 0);
            assert!((state.elapsed - sample["elapsed"].as_f64().unwrap() as f32).abs() < 1e-8);
            for (value, expected) in game
                .paths
                .enabled
                .iter()
                .zip(sample["paths"].as_array().unwrap())
            {
                assert_eq!(*value, expected.as_i64().unwrap() != 0);
            }
        }
        // The shared fractional clock expires this30ms switch before its own
        // accumulated float seconds reaches0.030, unlike the former float timer.
        assert!(!game.switches[0].enabled);
        assert!(game.switches[0].elapsed < 0.03);
    }
}

#[test]
fn executable_path_boundaries_share_collision_budget() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/physics");
    let reference: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("path-budget-results.json")).unwrap())
            .unwrap();
    let mut sol = empty();
    sol.paths = vec![path([0.; 3], 1, 1, 0), path([1., 0., 0.], 0, 1, 0)];
    for p in &mut sol.paths {
        p.duration = 0.006;
    }
    sol.bodies = vec![[0, -1, -1, 0, 0, 0, 0]];
    let mut paths = PathRuntime::new(&sol);
    let mut ball = BallState::new(vec3(0., 0.25, 0.), 0.25);
    ball.velocity = Vec3::X;
    let report = paths.step_ball(&sol, &PhysicsWorld::default(), &mut ball, Vec3::ZERO, 0.1);
    assert!(report.exhausted);
    assert_eq!(report.collisions, 0);
    for (actual, expected) in ball
        .position
        .to_array()
        .iter()
        .zip(reference["position"].as_array().unwrap())
    {
        assert_eq!(*actual, expected.as_f64().unwrap() as f32);
    }
    let mover = &paths.movers[0];
    assert_eq!(
        mover.elapsed,
        reference["mover"][0].as_f64().unwrap() as f32
    );
    assert_eq!(mover.millis, reference["mover"][1].as_i64().unwrap() as i32);
    assert_eq!(mover.path, reference["mover"][2].as_u64().unwrap() as usize);
    // Fifteen short boundaries then one final unconstrained advance. Resetting
    // a budget at every boundary would leave0.004 seconds on the next segment.
    assert_eq!(mover.elapsed, 0.);
}
