use std::io::Error;

#[derive(Clone, Copy)]
pub(crate) struct Note {
    pub frequency: f32,
    pub start: f64,
    pub end: f64,
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

pub(crate) fn save_midi(notes: &[Note], path: &str) -> Result<(), Error> {
    let mut bytes: Vec<u8> = Vec::new();
    let track_format: u16 = 1;
    let tracks_count: u16 = 1;
    let ticks_per_beat: u16 = 480;

    bytes.extend_from_slice(b"MThd"); // header
    bytes.extend_from_slice(&6u32.to_be_bytes());
    bytes.extend_from_slice(&track_format.to_be_bytes());
    bytes.extend_from_slice(&tracks_count.to_be_bytes());
    bytes.extend_from_slice(&ticks_per_beat.to_be_bytes());

    let mut track: Vec<u8> = Vec::new();
    // Tempo: 120 BPM -> 500000 microseconds per quarter
    track.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
    let bpm = 120.0f64;
    let sec_per_beat = 60.0f64 / bpm; // 0.5s
    let ticks_per_sec = ticks_per_beat as f64 / sec_per_beat; // 960
    let mut last_tick: f64 = 0.0;

    for note in notes {
        let freq = note.frequency as f64;
        let midi_note = (69.0 + 12.0 * (freq / 440.0).log2()).round() as u8;
        let start_tick = (note.start * ticks_per_sec) as i32;
        let delta = (start_tick - last_tick as i32) as u32;
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

    std::fs::write(path, bytes)
}
