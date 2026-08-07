use hound::WavReader;
use ndarray::Array2;
use std::error::Error;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
// use std::thread;
// use std::time::Duration;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

mod filter;

#[derive(Clone, Copy)]
struct Note {
    frequency: f32,
    start: f64,
    end: f64,
}

fn write_var_len(out: &mut Vec<u8>, mut value: u32) {
    // MIDI variable length quantity
    let mut buf = Vec::new();
    buf.push((value & 0x7F) as u8);
    value >>= 7;
    while value > 0 {
        buf.push(((value & 0x7F) as u8) | 0x80);
        value >>= 7;
    }
    for b in buf.iter().rev() {
        out.push(*b);
    }
}

fn save_midi(notes: &[Note], path: &str) {
    // Simple MIDI file writer (format 1, 1 track) with fixed tempo 120 BPM
    let mut bytes: Vec<u8> = Vec::new();
    // Header
    bytes.extend_from_slice(b"MThd");
    bytes.extend_from_slice(&6u32.to_be_bytes());
    bytes.extend_from_slice(&[0x00, 0x01]); // format 1
    bytes.extend_from_slice(&[0x00, 0x01]); // 1 track
    bytes.extend_from_slice(&[0x01, 0xE0]); // 480 ticks per beat

    // Track data
    let mut track: Vec<u8> = Vec::new();
    // Tempo: 120 BPM -> 500000 microseconds per quarter
    track.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
    let ticks_per_beat = 480f64;
    let bpm = 120.0f64;
    let sec_per_beat = 60.0f64 / bpm; // 0.5s
    let ticks_per_sec = ticks_per_beat / sec_per_beat; // 960
    let mut last_tick: f64 = 0.0;

    for note in notes {
        let freq = note.frequency as f64;
        let midi_note = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
        let start_tick = (note.start * ticks_per_sec) as i32;
        let delta = (start_tick as i32 - last_tick as i32) as u32;
        write_var_len(&mut track, delta);
        last_tick = start_tick as f64;
        track.push(0x90); // note_on, channel 0
        track.push(midi_note);
        track.push(0x64); // velocity 100
        let end_tick = (note.end * ticks_per_sec) as i32;
        let delta2 = (end_tick - last_tick as i32) as u32;
        write_var_len(&mut track, delta2);
        last_tick = end_tick as f64;
        track.push(0x80); // note_off
        track.push(midi_note);
        track.push(0x40); // velocity 64
    }
    // End of track
    track.push(0x00);
    track.extend_from_slice(&[0xFF, 0x2F, 0x00]);

    bytes.extend_from_slice(b"MTrk");
    bytes.extend_from_slice(&(track.len() as u32).to_be_bytes());
    bytes.extend_from_slice(&track);

    std::fs::write(path, bytes).expect("failed to write midi");
}

fn main() -> Result<(), Box<dyn Error>> {
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

    // stream.play().expect("failed to start stream");

    // // Record for a fixed duration (scaffold)
    // thread::sleep(Duration::from_secs(5));
    // drop(stream);

    let audio = samples.lock().unwrap().clone();
    let duration = audio.len() as f64 / sample_rate as f64;

    let (samples, sample_rate_hz) = filter::load_input()?;
    let f = filter::cqt(&samples, sample_rate_hz)?;
    println!("{f}");

    let notes = vec![Note {
        frequency: 440.0f32,
        start: 0.0,
        end: duration,
    }];

    save_midi(&notes, "output.mid");
    Ok(())
}
