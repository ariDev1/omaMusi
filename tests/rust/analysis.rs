use omamusi::{
    audio::PcmSnapshot,
    visual::{
        VisualMode, VisualState, analysis::MusicMetrics, director::VisualDirector,
        waveform::WaveformPersistence,
    },
};
use serde_json::Value;

fn near(actual: f64, expected: f64, tolerance: f64, name: &str) {
    assert!(
        (actual - expected).abs() <= tolerance * (1.0 + expected.abs()),
        "{name}: {actual} != {expected}"
    );
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/visual-reference.json")).unwrap()
}
fn scalar(frame: &omamusi::visual::VisualFrame, name: &str) -> f64 {
    match name {
        "energy" => frame.energy,
        "bass" => frame.bass,
        "treble" => frame.treble,
        "time" => frame.time,
        "phi_pulse" => frame.phi_pulse,
        "phi_bloom" => frame.phi_bloom,
        "phi_tension" => frame.phi_tension,
        "phi_event" => frame.phi_event,
        "phi_velocity" => frame.phi_velocity,
        "phi_impulse" => frame.phi_impulse,
        "aether_onset" => frame.aether_onset,
        "aether_beat_phase" => frame.aether_beat_phase,
        "aether_beat_confidence" => frame.aether_beat_confidence,
        "aether_density" => frame.aether_density,
        "aether_scene_morph" => frame.aether_scene_morph,
        "aether_world_turn" => frame.aether_world_turn,
        "aether_beat_pulse" => frame.aether_beat_pulse,
        "aether_phrase_progress" => frame.aether_phrase_progress,
        _ => panic!("unknown oracle field {name}"),
    }
}

#[test]
fn analysis_matches_python_oracle_across_tones_rates_and_paused_frames() {
    let data = fixture();
    let mut state = VisualState::new();
    state.mode = VisualMode::Waveform;
    for frame in data["frames"].as_array().unwrap() {
        let rate = frame["rate"].as_u64().unwrap() as u32;
        let frequency = frame["frequency"].as_f64().unwrap();
        let amplitude = frame["amplitude"].as_f64().unwrap();
        let index = frame["index"].as_u64().unwrap();
        let samples = std::array::from_fn(|i| {
            (amplitude * (std::f64::consts::TAU * frequency * i as f64 / rate as f64).sin()) as f32
        });
        if frame["fresh"].as_bool().unwrap() {
            state.feed(&PcmSnapshot {
                samples,
                sample_rate: rate,
                revision: index + 1,
                generation: 1,
            });
        }
        state.tick(
            data["dt"].as_f64().unwrap(),
            frame["active"].as_bool().unwrap(),
        );
        let actual = state.frame();
        for (name, expected) in frame["values"].as_object().unwrap() {
            near(
                scalar(actual, name),
                expected.as_f64().unwrap(),
                2e-5,
                &format!("frame {index} {name}"),
            );
        }
        for (key, values) in [("bands", &actual.bands), ("peaks", &actual.peaks)] {
            for (i, want) in frame[key].as_array().unwrap().iter().enumerate() {
                near(
                    values[i],
                    want.as_f64().unwrap(),
                    2e-4,
                    &format!("frame {index} {key}[{i}]"),
                );
            }
        }
        for (actual, expected) in actual.waves.iter().flatten().zip(
            frame["waves"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|v| v.as_array().unwrap()),
        ) {
            near(*actual as f64, expected.as_f64().unwrap(), 2e-5, "waves");
        }
        for (actual, expected) in actual.spots.iter().flatten().zip(
            frame["spots"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|v| v.as_array().unwrap()),
        ) {
            near(*actual as f64, expected.as_f64().unwrap(), 2e-5, "spots");
        }
        near(
            state.waveform.density.iter().map(|v| *v as f64).sum(),
            frame["wave_density_sum"].as_f64().unwrap(),
            2e-5,
            "wave density sum",
        );
        near(
            state
                .waveform
                .density
                .iter()
                .copied()
                .fold(0.0f32, f32::max) as f64,
            frame["wave_density_max"].as_f64().unwrap(),
            2e-5,
            "wave density max",
        );
        near(
            state.waveform.latest_alpha,
            frame["wave_alpha"].as_f64().unwrap(),
            2e-5,
            "wave alpha",
        );
        for row in 0..96 {
            for channel in 0..3 {
                let actual = actual.history[(row * 360 + 359) * 3 + channel];
                let expected = frame["history_tail"][row][channel].as_u64().unwrap() as u8;
                assert!(
                    actual.abs_diff(expected) <= 1,
                    "history frame {index} row {row}"
                );
            }
        }
    }
}

#[test]
fn director_matches_reference_phrase_changes() {
    let data = fixture();
    let mut director = VisualDirector::default();
    for (index, expected) in data["director"].as_array().unwrap().iter().enumerate() {
        let metrics = MusicMetrics {
            onset: 0.6,
            beat_phase: if index % 2 == 0 { 0.9 } else { 0.1 },
            beat_confidence: 0.8,
            density: 0.4,
            high_transient: 0.2,
            ..Default::default()
        };
        let actual = director.update(metrics, 1.0 / 60.0);
        assert_eq!(
            actual.scene_index,
            expected["scene_index"].as_u64().unwrap() as usize
        );
        for (name, value) in [
            ("scene_morph", actual.scene_morph),
            ("world_turn", actual.world_turn),
            ("beat_pulse", actual.beat_pulse),
            ("phrase_progress", actual.phrase_progress),
        ] {
            near(value, expected[name].as_f64().unwrap(), 2e-8, name);
        }
    }
}

#[test]
fn waveform_density_and_pixels_match_oracle_and_decay_without_new_audio() {
    let mut wave = WaveformPersistence::new();
    let trace: Vec<f32> = (0..1024)
        .map(|i| (-0.5 + i as f64 / 1023.0) as f32)
        .collect();
    wave.step(1.0 / 60.0, Some(&trace), None);
    for probe in fixture()["wave_probes"].as_array().unwrap() {
        let offset = probe["row"].as_u64().unwrap() as usize * 768
            + probe["column"].as_u64().unwrap() as usize;
        near(
            wave.density[offset] as f64,
            probe["density"].as_f64().unwrap(),
            2e-5,
            "wave probe",
        );
        for channel in 0..4 {
            assert!(
                wave.rgba()[offset * 4 + channel]
                    .abs_diff(probe["rgba"][channel].as_u64().unwrap() as u8)
                    <= 1
            );
        }
    }
    let before: f64 = wave.density.iter().map(|v| *v as f64).sum();
    wave.step(0.5, None, None);
    let after: f64 = wave.density.iter().map(|v| *v as f64).sum();
    near(after / before, (-1.0f64).exp(), 2e-6, "decay");
    assert_eq!(&wave.rgba()[..4], [0, 0, 0, 0]);
}

#[test]
fn repeated_audio_revisions_do_not_deposit_and_visual_cycles_preserve_stars() {
    let mut visual = VisualState::new();
    let original = visual.particle_vertices.clone();
    visual.mode = VisualMode::Waveform;
    let snapshot = PcmSnapshot {
        samples: [0.1; 4096],
        sample_rate: 48000,
        revision: 1,
        generation: 4,
    };
    visual.feed(&snapshot);
    visual.tick(1.0 / 60.0, true);
    let before: f64 = visual.waveform.density.iter().map(|v| *v as f64).sum();
    visual.feed(&snapshot);
    visual.tick(1.0 / 60.0, true);
    let after: f64 = visual.waveform.density.iter().map(|v| *v as f64).sum();
    assert!(after < before);
    for _ in 0..9 {
        visual.cycle(1);
    }
    assert_eq!(visual.mode, VisualMode::Waveform);
    visual.cycle(-1);
    assert_eq!(visual.mode, VisualMode::Spectrum);
    assert_eq!(visual.particle_vertices, original);
    visual.reset();
    assert!(visual.frame().bands.iter().all(|v| *v == 0.0));
    assert!(visual.waveform.density.iter().all(|v| *v == 0.0));
    assert_eq!(visual.particle_vertices, original);
}

#[test]
fn invalid_samples_and_time_do_not_poison_visual_state() {
    let mut visual = VisualState::new();
    let mut samples = [f32::NAN; 4096];
    samples[1] = f32::INFINITY;
    visual.feed(&PcmSnapshot {
        samples,
        sample_rate: 0,
        revision: 1,
        generation: 1,
    });
    visual.tick(f64::NAN, true);
    assert!(visual.frame().bands.iter().all(|v| v.is_finite()));
    assert!(visual.frame().energy.is_finite());
}

#[test]
fn beat_estimation_matches_reference_after_repeated_half_second_pulses() {
    let expected: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/beat-reference.json")).unwrap();
    let mut analyzer = omamusi::visual::analysis::MusicAnalyzer::new();
    for (index, expected) in expected.iter().enumerate() {
        let hit = index % 25 == 0;
        let mut bands = [0.0; 96];
        if hit {
            bands[..18].fill(0.8);
        }
        let actual = analyzer.update(
            0.02,
            &bands,
            if hit { 0.4 } else { 0.0 },
            if hit { 0.8 } else { 0.0 },
            if hit { 0.1 } else { 0.0 },
        );
        for (name, value) in [
            ("onset", actual.onset),
            ("beat_phase", actual.beat_phase),
            ("beat_confidence", actual.beat_confidence),
            ("bpm", actual.bpm),
            ("density", actual.density),
            ("low_transient", actual.low_transient),
        ] {
            near(value, expected[name].as_f64().unwrap(), 2e-5, name);
        }
    }
}

#[test]
fn horizon_slots_preserve_identities_when_old_hits_expire() {
    use omamusi::visual::horizon::{HorizonHotspots, wave_payload};
    let mut spots = HorizonHotspots::default();
    let hit = MusicMetrics {
        mid_transient: 0.4,
        ..Default::default()
    };
    spots.update(0.016, hit, false);
    assert!(spots.payload().iter().all(|s| s[0] < 0.0));
    for _ in 0..5 {
        spots.update(0.25, hit, true);
    }
    let old = spots.payload();
    spots.update(0.25, hit, true);
    let new = spots.payload();
    for i in 0..3 {
        assert_eq!(new[i][2], old[i + 1][2]);
    }
    spots.update(4.3, MusicMetrics::default(), false);
    assert!(spots.payload().iter().all(|s| s[0] < 0.0));
    let waves = wave_payload(&[[0.1, 0.2], [0.2, 0.3]]);
    assert_eq!(waves[0], [0.1, 0.2]);
    assert_eq!(waves[1], [0.2, 0.3]);
    assert!(waves[2..].iter().all(|w| w[0] < 0.0));
}

#[test]
fn warp_and_phi_receive_live_waveform_data_without_visiting_waveform_mode() {
    for mode in [VisualMode::Warp, VisualMode::PhiCathedral] {
        let mut visual = VisualState::new();
        visual.mode = mode;
        visual.feed(&PcmSnapshot {
            samples: std::array::from_fn(|i| (0.2 * (i as f64 * 0.1).sin()) as f32),
            sample_rate: 48000,
            revision: 1,
            generation: 1,
        });
        visual.tick(0.016, true);
        assert!(visual.frame().waveform.iter().any(|v| v.abs() > 0.1));
    }
}
