// SPDX-License-Identifier: GPL-3.0-or-later
use reagent_neverball_rs::flow::*;
use reagent_neverball_rs::replay::{Frame, Playback, Replay, ReplayStore};
use reagent_neverball_rs::settings::Settings;
fn flow() -> Flow {
    Flow::new(
        vec![SetSpec {
            id: "easy".into(),
            levels: vec![
                LevelSpec {
                    id: "one".into(),
                    bonus: false,
                },
                LevelSpec {
                    id: "bonus".into(),
                    bonus: true,
                },
                LevelSpec {
                    id: "two".into(),
                    bonus: false,
                },
            ],
        }],
        Progress::default(),
    )
}
#[test]
fn observed_challenge_timeout_retry_loses_one_spare_and_preserves_pause() {
    // Runtime flow-07 → flow-timeout-10 → flow-pause-11: HUD Balls 2 → 1.
    let mut f = flow();
    f.start(0, 0, Mode::Challenge).unwrap();
    assert_eq!(f.session.as_ref().unwrap().balls - 1, 2);
    f.begin_play();
    f.finish(Outcome::TimeOut, 12, 90_000, "Player").unwrap();
    assert_eq!(f.session.as_ref().unwrap().coins, 0);
    f.retry().unwrap();
    f.begin_play();
    assert!(f.pause());
    assert!(f.resume());
    assert_eq!(f.session.as_ref().unwrap().balls - 1, 1);
    assert!(f.finish(Outcome::FallOut, 0, 1000, "Player").is_ok());
    f.retry().unwrap();
    f.begin_play();
    f.finish(Outcome::FallOut, 0, 1000, "Player").unwrap();
    assert_eq!(f.screen, Screen::GameOver);
    assert!(f.retry().is_err());
}
#[test]
fn binary_result_rules_bank_success_award_hundreds_and_retry_rolls_back() {
    // FUN_20ee0 banks only success, awards each100; FUN_20be0 restores pre-success totals.
    let mut f = flow();
    f.start(0, 0, Mode::Challenge).unwrap();
    f.begin_play();
    let r = f.finish(Outcome::Success, 205, 1200, "Player").unwrap();
    assert_eq!(r.extra_balls, 2);
    assert_eq!(f.session.as_ref().unwrap().balls, 5);
    assert!(f.unlocked(0, 1));
    assert!(f.unlocked(0, 2));
    assert_eq!(f.result.as_ref().unwrap().next_level, Some(2));
    f.retry().unwrap();
    assert_eq!(f.session.as_ref().unwrap().coins, 0);
    assert_eq!(f.session.as_ref().unwrap().balls, 3);
    f.begin_play();
    f.finish(Outcome::Success, 99, 1300, "Player").unwrap();
    f.next().unwrap();
    f.begin_play();
    f.finish(Outcome::Success, 1, 900, "Player").unwrap();
    assert_eq!(f.session.as_ref().unwrap().balls, 4);
    assert!(f.next().unwrap().is_none());
    assert_eq!(f.screen, Screen::SetComplete);
}
#[test]
fn normal_skips_locked_bonus_and_goal_choice_only_applies_to_completed_levels() {
    let mut f = flow();
    assert!(f.start(0, 2, Mode::Normal).is_err());
    f.lock_completed_goals = false;
    assert!(!f.start(0, 0, Mode::Normal).unwrap().goal_open);
    f.begin_play();
    f.finish(Outcome::Success, 10, 5000, "Player").unwrap();
    assert!(!f.unlocked(0, 1));
    assert!(f.unlocked(0, 2));
    assert!(f.retry().unwrap().goal_open);
    let saved = f.progress.to_json().unwrap();
    let loaded = Flow::new(f.sets.clone(), Progress::from_json(&saved).unwrap());
    assert!(loaded.unlocked(0, 2));
}
#[test]
fn ranking_binary_ties_use_coin_then_time_and_keep_three() {
    let mut r = Records::default();
    for (time, coins) in [(20, 3), (10, 2), (10, 4), (30, 4)] {
        r.submit(
            Record {
                player: "P".into(),
                elapsed_ms: time,
                coins,
            },
            false,
        );
    }
    assert_eq!(
        r.best_times
            .iter()
            .map(|x| (x.elapsed_ms, x.coins))
            .collect::<Vec<_>>(),
        [(10, 4), (10, 2), (20, 3)]
    );
    assert_eq!(
        r.most_coins
            .iter()
            .map(|x| (x.elapsed_ms, x.coins))
            .collect::<Vec<_>>(),
        [(10, 4), (30, 4), (20, 3)]
    );
    let len = r.fast_unlock.len();
    r.submit(
        Record {
            player: "P".into(),
            elapsed_ms: 1,
            coins: 10,
        },
        true,
    );
    assert_eq!(r.fast_unlock.len(), len);
}
fn frame(at: u32, x: f32) -> Frame {
    Frame {
        at_ms: at,
        position: [x, 0., 0.],
        orientation: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        radius: 0.25,
        camera_eye: [0., 2., 3.],
        camera_target: [x, 0., 0.],
        tilt: [0., 0.],
        coins: 0,
        remaining_seconds: 90.,
        state: serde_json::json!({"collected":[false,true]}),
    }
}
#[test]
fn saved_replay_plays_exact_snapshots_pauses_ends_and_roundtrips() {
    let mut r = Replay::new("easy", "one", "Player");
    r.push(frame(0, 1.)).unwrap();
    r.push(frame(20, 2.)).unwrap();
    r.push(frame(40, 3.)).unwrap();
    assert!(r.push(frame(40, 5.)).is_err());
    let mut store = ReplayStore::default();
    store.set_last(r);
    store.save_last("run", false).unwrap();
    assert!(store.save_last("run", false).is_err());
    let restored = ReplayStore::from_json(&store.to_json().unwrap()).unwrap();
    let mut play = Playback::new(restored.saved["run"].clone());
    assert_eq!(play.advance(0.025).unwrap().position[0], 2.);
    play.paused = true;
    assert_eq!(play.advance(1.).unwrap().position[0], 2.);
    play.paused = false;
    assert_eq!(play.advance(1.).unwrap().position[0], 3.);
    assert!(play.ended);
    play.restart();
    assert_eq!(play.advance(0.).unwrap().position[0], 1.);
    assert!(store.delete("run"));
    assert!(store.last.is_some());
}
#[test]
fn runtime_settings_keep_multiword_bindings_unknown_fields_and_audio_bounds() {
    let s = Settings::from_rc(
        "camera 2\nsound_volume 10\nmusic_volume 6\nkey_rotate_fast Left Shift\nfuture_field 123\n",
    )
    .unwrap();
    assert_eq!(s.bindings["key_rotate_fast"], "Left Shift");
    assert_eq!(s.camera, 3); // rc2 = UI3/Manual, not UI2/Lazy.
    assert_eq!(Settings::from_rc("camera 0").unwrap().camera, 1);
    assert_eq!(s.sound_gain(), 1.);
    assert_eq!(s.music_gain(), 0.6);
    assert_eq!(Settings::from_rc(&s.to_rc()).unwrap(), s);
    assert_eq!(Settings::from_json(&s.to_json().unwrap()).unwrap(), s);
    assert!(Settings::from_rc("sound_volume 11").is_err());
}

#[test]
fn imports_original_nbr_camera_traces_exactly() {
    use reagent_neverball_rs::replay::OriginalReplay;
    let root = original_replay_fixtures();
    for name in ["camera-modes", "camera-transitions"] {
        let bytes = std::fs::read(root.join(format!("{name}.nbr"))).unwrap();
        let imported = OriginalReplay::from_nbr(&bytes).unwrap();
        assert_eq!(imported.to_nbr().unwrap(), bytes);
        let expected: Vec<serde_json::Value> = serde_json::from_str(
            &std::fs::read_to_string(root.join(format!("{name}.camera.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(imported.batches.len(), expected.len());
        assert_eq!(imported.replay.frames.len(), expected.len());
        for (frame, row) in imported.replay.frames.iter().zip(&expected) {
            for (key, actual) in [
                ("position", frame.position),
                ("eye", frame.camera_eye),
                ("target", frame.camera_target),
            ] {
                for (i, v) in actual.iter().enumerate() {
                    assert_eq!(*v, row[key][i].as_f64().unwrap() as f32, "{name} {key} {i}");
                }
            }
            assert_eq!(
                frame.at_ms,
                (row["t"].as_f64().unwrap() * 1000.).round() as u32
            );
            assert_eq!(
                frame.remaining_seconds,
                row["timer"].as_f64().unwrap() as f32
            );
        }
        assert!(
            imported.unhandled_kinds.is_empty(),
            "Unhandled commands: {:?}",
            imported.unhandled_kinds
        );
        for cut in [0, 3, 20, bytes.len() - 1] {
            assert!(OriginalReplay::from_nbr(&bytes[..cut]).is_err());
        }
    }
}

#[test]
fn native_save_roundtrip_retains_progress_settings_and_named_replay() {
    use reagent_neverball_rs::settings::SaveData;
    let root =
        std::env::temp_dir().join(format!("neverball-flow-save-test-{}", std::process::id()));
    let path = root.join("save.json");
    let mut save = SaveData::new();
    save.settings.player = "Test Player".into();
    save.progress = flow().progress;
    let mut replay = Replay::new("easy", "one", "Test Player");
    replay.push(frame(0, 1.)).unwrap();
    save.replays.set_last(replay);
    save.replays.save_last("test", false).unwrap();
    save.save(&path).unwrap();
    let loaded = SaveData::load(&path).unwrap();
    assert_eq!(loaded.settings.player, "Test Player");
    assert!(loaded.progress.sets["easy"].unlocked.contains("one"));
    assert!(loaded.replays.saved.contains_key("test"));
    assert!(!path.with_extension("json.tmp").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn original_item_commands_remove_the_observed_items_and_update_coin_total() {
    use reagent_neverball_rs::replay::OriginalReplay;
    let root = original_replay_fixtures();
    let bytes = std::fs::read(root.join("holdout-diagonal.nbr")).unwrap();
    let original = OriginalReplay::from_nbr(&bytes).unwrap();
    assert_eq!(original.to_nbr().unwrap(), bytes);
    // Actual command log: item15/coin1 at7.75556s; item4/coin2 at12.12222s.
    let before = original.replay.frame_at(7700).unwrap();
    assert_eq!(before.coins, 0);
    let first = original.replay.frame_at(7800).unwrap();
    assert_eq!(first.coins, 1);
    assert_eq!(first.state["collected"][15], true);
    let second = original.replay.frame_at(12250).unwrap();
    assert_eq!(second.coins, 2);
    assert_eq!(second.state["collected"][4], true);
    assert_eq!(second.state["collected"][15], true);
    assert!(original.unhandled_kinds.is_empty());
}

#[test]
fn original_mover_replay_applies_recorded_path_boundaries_and_clock_resets() {
    use reagent_neverball_rs::{replay::OriginalReplay, sol::Sol};
    let root = original_replay_fixtures();
    let bytes = std::fs::read(root.join("physics-mover-complete-batches.nbr")).unwrap();
    let sol = Sol::from_bytes(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data/map-easy/mover.sol"),
        )
        .unwrap(),
    )
    .unwrap();
    let original = OriginalReplay::from_nbr(&bytes)
        .unwrap()
        .with_sol(&sol)
        .unwrap();
    assert_eq!(original.header.level, "map-easy/mover.sol");
    assert_eq!(original.batches.len(), 889);
    assert_eq!(original.to_nbr().unwrap(), bytes);
    // Raw reference commands reset mover0 then select path1 at5.001s,
    // and reset it then select path2 at6.001s. This is not automatic link following.
    for (at, path, start) in [(4900, 0, 0.0), (5500, 1, 5.001), (6500, 2, 6.001)] {
        let frame = original.replay.frame_at(at).unwrap();
        let mover = &frame.state["movers"][0];
        assert_eq!(mover["path"], path);
        let elapsed = mover["elapsed"].as_f64().unwrap();
        assert!(
            (elapsed - (frame.at_ms as f64 / 1000. - start)).abs() < 0.002,
            "{at}: {elapsed}"
        );
        assert_eq!(
            frame.state["body_poses"].as_array().unwrap().len(),
            sol.bodies.len()
        );
    }
    assert!(original.unhandled_kinds.is_empty());
}

#[test]
fn compiled_easy_level_record_thresholds_seed_rankings() {
    use reagent_neverball_rs::{content, sol::Sol};
    let mut sol = Sol::default();
    // Actual compiled map-easy/easy.sol metadata, plus binary two-value defaults.
    for (k, v) in [
        ("time", "9000"),
        ("goal", "10"),
        ("time_hs", "350 500"),
        ("coin_hs", "18 15"),
        ("goal_hs", "2000 4500"),
    ] {
        sol.metadata.insert(k.into(), v.into());
    }
    let records =
        Records::reference_level(&content::LevelSpec::from_sol("map-easy/easy.sol", &sol));
    assert_eq!(
        records
            .best_times
            .iter()
            .map(|r| r.elapsed_ms)
            .collect::<Vec<_>>(),
        [3500, 5000, 90000]
    );
    assert_eq!(
        records
            .fast_unlock
            .iter()
            .map(|r| r.elapsed_ms)
            .collect::<Vec<_>>(),
        [20000, 45000, 90000]
    );
    assert_eq!(
        records
            .most_coins
            .iter()
            .map(|r| r.coins)
            .collect::<Vec<_>>(),
        [18, 15, 10]
    );
    assert_eq!(records.best_times[0].player, "Hard");
    assert_eq!(records.best_times[2].player, "Easy");
}

fn original_replay_fixtures() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/original-replays")
}

#[test]
fn original_teleport_replay_retains_camera_batches_while_physics_is_frozen() {
    use reagent_neverball_rs::replay::OriginalReplay;
    let bytes = std::fs::read(original_replay_fixtures().join("easytele.nbr")).unwrap();
    let replay = OriginalReplay::from_nbr(&bytes).unwrap();
    assert_eq!(replay.batches.len(), 601);
    assert_eq!(replay.replay.frames.len(), 601);
    assert_eq!(replay.to_nbr().unwrap(), bytes);
    assert_eq!(replay.replay.frames.last().unwrap().at_ms, 6667);
    // The original records ninety camera/timer batches without cmd30 during a jump.
    for index in 155..245 {
        assert!(!replay.batches[index].commands.iter().any(|c| c.kind == 30));
        assert_eq!(
            replay.replay.frames[index].at_ms,
            (index as f64 / 90. * 1000.).round() as u32
        );
    }
    assert_ne!(
        replay.replay.frames[155].camera_eye,
        replay.replay.frames[244].camera_eye
    );
    assert!(replay
        .replay
        .frames
        .windows(2)
        .all(|pair| pair[1].at_ms > pair[0].at_ms));
}

#[test]
fn editable_bindings_preserve_native_aliases_and_roundtrip() {
    let mut settings =
        Settings::from_rc("key_forward Up\nkey_camera_1 1\nkey_rotate_fast Left Shift\n").unwrap();
    assert_eq!(settings.binding_name("key_camera_1"), "Key1");
    assert_eq!(settings.binding_name("key_rotate_fast"), "LeftShift");
    settings.language = "fr".into();
    settings.set_key_binding("key_forward", "w").unwrap();
    settings.set_key_binding("key_camera_1", "F1").unwrap();
    assert!(settings.set_key_binding("key_forward", "Escape").is_err());
    assert!(settings.set_key_binding("not_an_action", "W").is_err());
    let loaded = Settings::from_json(&settings.to_json().unwrap()).unwrap();
    assert_eq!(loaded.language, "fr");
    assert_eq!(loaded.binding_name("key_forward"), "W");
    assert_eq!(loaded.binding_name("key_camera_1"), "F1");
}
