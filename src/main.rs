use cpal::traits::StreamTrait;
use cpal::traits::{DeviceTrait, HostTrait};
use getch_rs::{Getch, Key};
use std::error::Error;
use std::f32::consts::PI;
use std::process::exit;
use std::sync::atomic::AtomicU32;
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
            Key::Ctrl('c') => {
                exit(0);
            }
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

    let device = host
        .default_output_device()
        .expect("no input device available");

    let config = device.default_output_config()?.config();
    let sample_rate = config.sample_rate as f32;
    let channels = config.channels as usize;

    let freq_lock = Arc::new(AtomicU32::new(0));
    let freq_lock_clone = freq_lock.clone();

    let err_fn = |err: cpal::Error| match err.kind() {
        cpal::ErrorKind::DeviceChanged
        | cpal::ErrorKind::Xrun
        | cpal::ErrorKind::RealtimeDenied => {
            eprintln!("{err}")
        }
        _ => eprintln!("Stream error: {err}"),
    };

    let mut phase = 0.0_f32;
    let stream = device.build_output_stream(
        config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            for frame in data.chunks_mut(channels) {
                let freq = freq_lock_clone.load(std::sync::atomic::Ordering::Relaxed);
                let freq = f32::from_bits(freq);

                let value = (2.0 * PI * phase).sin() * 0.25;

                phase += freq / sample_rate;

                if phase >= 1.0 {
                    phase -= 1.0;
                }

                for sample in frame.iter_mut() {
                    *sample = value;
                }
            }
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    println!(
        "Replaying the notes back to you. Press any key to mark the rhythm or control c to exit"
    );

    let now = std::time::Instant::now();

    let mut freq_index = 0;
    let mut durations = vec![];
    while let Ok(key) = g.getch() {
        match key {
            Key::Ctrl('c') => {
                break;
            }
            _ => {
                if let Some(&freq) = freqs.get(freq_index) {
                    freq_lock.store(
                        (freq as f32 * 4.0).to_bits(),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    freq_index += 1;
                    durations.push(now.elapsed().as_secs_f32());
                } else {
                    durations.push(now.elapsed().as_secs_f32());
                    break;
                }
            }
        }
    }

    let mut freq_start_end = freqs.iter().zip(durations.windows(2));
    while let Some((&freq, &[start, end])) = freq_start_end.next() {
        notes.push(Note {
            frequency: freq as f32 * 4.0,
            start: start as f64,
            end: end as f64,
        });
    }

    save_midi(&notes, "output.mid")?;
    Ok(())
}
