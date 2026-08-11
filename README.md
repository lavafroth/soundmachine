# Sound Machine

Convert hummed melody into MIDI.

Single binary rust port of [hum](https://github.com/lavafroth/hum).

## Getting Started

```sh
nix develop
cargo run --release
```

### Capturing Frequencies

For the first pass, hum each note *slowly* and press the space bar at the start of each note.
Press _Q_ to confirm.

### Rhythm `TODO`

After pressing _Q_, the frequencies will be played back to you.

Press any key on the keyboard to time the rhythm of the notes.
Once the melody plays out, the program outputs a MIDI file `output.mid`.

### Replay

You can play the output MIDI file with a command line tool like _timidity_

```sh
timidity --volume=150 output.mid
```

or open them in a digital audio workstation like _Ardour_.
