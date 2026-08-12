use cpal::traits::StreamTrait;
use cpal::traits::{DeviceTrait, HostTrait};
use getch_rs::{Getch, Key};
use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};

mod filter;
mod midi;
use midi::{Note, save_midi};

fn main() -> Result<(), Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .expect("no input device available");
    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate();

    let samples = Arc::new(Mutex::new(Vec::<f32>::new()));
    let samples_clone = samples.clone();

    let err_fn = |err| eprintln!("stream error: {}", err);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            config.config(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                samples_clone.lock().unwrap().extend_from_slice(data);
            },
            err_fn,
            None,
        )?,

        _ => {
            eprintln!("Unsupported sample format for input audio stream, exiting.");
            exit(1);
        }
    };

    let g = Getch::new();

    println!("Press q to finish recording, any other key to mark a note boundary.");

    stream.play()?;
    let mut chunk_segments = vec![]; // 0 to first chunk is discarded
    while let Ok(key) = g.getch() {
        match key {
            Key::Char('q') => {
                stream.pause()?;
                chunk_segments.push(samples.lock().unwrap().len());
                break;
            }

            _ => {
                chunk_segments.push(samples.lock().unwrap().len());
            }
        }
    }

    let samples = samples.lock().unwrap();
    let mut notes = vec![];
    let mut freqs = vec![];
    let sample_rate_hz = sample_rate as f64;

    let mut windows = chunk_segments.windows(2);
    while let Some(&[start, end]) = windows.next() {
        let freq = filter::cqt(&samples[start..end], sample_rate_hz)?;
        freqs.push(freq);
    }

    let mut last = 0.0;
    for freq in freqs {
        notes.push(Note {
            frequency: freq as f32 * 4.0,
            start: last,
            end: last + 1.0,
        });

        last += 1.0;
    }

    save_midi(&notes, "output.mid")?;
    Ok(())
}
