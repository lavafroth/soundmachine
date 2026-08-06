use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

#[derive(Clone, Copy)]
struct Note {
    frequency: f32,
    start: f64,
    end: f64,
}

fn estimate_pitch_from_audio(_samples: &[f32], _sample_rate: u32) -> f32 {
    // TODO: replace with spectrograms-based detector
    440.0
}

fn save_midi(_notes: &[Note], _path: &str) {
    println!("Scaffold: would save MIDI to {}", _path);
}

fn main() {
    // Audio capture with CPAL
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("no input device available");
    let config = device
        .default_input_config()
        .expect("failed to get default input config");
    let sample_rate = config.sample_rate();

    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let samples_clone = samples.clone();

    let err_fn = |err| eprintln!("stream error: {}", err);

    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_input_stream(
                config.config(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    samples_clone.lock().unwrap().extend_from_slice(data);
                },
                err_fn,
                None,
            )
            .expect("failed to build input stream"),
        _ => panic!("Unsupported sample format"),
    };

    stream.play().expect("failed to start stream");

    // Record for a fixed duration (scaffold)
    thread::sleep(Duration::from_secs(5));
    drop(stream);

    let audio = samples.lock().unwrap().clone();
    let duration = audio.len() as f64 / sample_rate as f64;
    let freq = estimate_pitch_from_audio(&audio, sample_rate);

    let notes = vec![Note {
        frequency: freq,
        start: 0.0,
        end: duration,
    }];

    save_midi(&notes, "output.mid");
}
