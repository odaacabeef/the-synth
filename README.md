# the-synth

A multi-instance MIDI synthesizer for the terminal. Run multiple synthesizers
simultaneously, each with 16-voice polyphony, independent ADSR envelope control,
and configurable MIDI/audio channel routing.

## Usage

```bash
# List available MIDI and audio devices
the-synth --list

# Run with configuration file
the-synth --config config.yaml
```

## Configuration

Create a YAML configuration file specifying devices and synthesizer instances:

```yaml
devices:
  midiin: "Your MIDI Device"    # Name or index from --list
  audioout: "Your Audio Device" # Name or index from --list

synths:
  - midich: 1           # MIDI channel 1-16 or "omni"
    audioch: 1          # Audio output channel (1-indexed)
    attack: 0.01
    decay: 0.1
    sustain: 0.4
    release: 0.1
    wave: sine          # sine, triangle, sawtooth, square

  - midich: 2
    audioch: 2
    attack: 0.001
    decay: 0.05
    sustain: 0.7
    release: 0.2
    wave: sawtooth
```

Each synth instance has 16-voice polyphony and independent ADSR/waveform settings.

## Interface

![screenshot](docs/screenshot.png)

**Attack** (0.001s - 2.0s): Time to reach peak level when a note is pressed

**Decay** (0.001s - 2.0s): Time to fall from peak to sustain level

**Sustain** (0.0 - 1.0): Amplitude level held while note is pressed

**Release** (0.001s - 5.0s): Time to fade to silence after note is released

**Waveform**: Oscillator shape (Sine, Triangle, Sawtooth, Square)

The bottom section shows active voices - each of the 16 polyphonic voices
displays its current note (e.g., "C4", "F#3") or "---" when idle.

## Controls

```
j/k, ↑/↓      = Move cursor between parameters

h/l, ←/→      = Adjust the selected parameter value

tab/shift+tab = Switch between synth instances

?             = Toggle help

q, ctrl+c     = Quit
```
