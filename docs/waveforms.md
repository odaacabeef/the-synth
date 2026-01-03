# Waveforms

the-synth provides four classic waveform types, each with distinct sonic
characteristics.

**Sine:** A pure tone containing only the fundamental frequency with no
harmonics. Produces a smooth, mellow sound. Mathematically defined as `sin(2πφ)`
where φ is the phase (0.0 to 1.0).

**Triangle:** A waveform with linear rise and fall, containing odd harmonics
that decay at 1/n². Softer and mellower than sawtooth, with a hollow or woody
character.

**Sawtooth:** A linear ramp waveform containing all harmonics (both odd and
even) that decay at 1/n. Produces a bright, buzzy, and rich sound. Excellent for
brass and string timbres.

**Square:** Alternates between maximum positive and negative values, containing
odd harmonics that decay at 1/n. Creates a hollow, clarinet-like sound with
strong harmonic content.

## Perceptual Loudness

While all waveforms generate signals with the same peak amplitude (-1.0 to
+1.0), they differ significantly in perceived loudness:

**Quietest to Loudest:**
1. Sine (only fundamental frequency)
2. Triangle (gentle odd harmonics)
3. Sawtooth (rich harmonic content)
4. Square (continuous peak amplitude + strong harmonics)

This loudness variation occurs because:

- **Harmonic content**: Waveforms with more harmonics distribute energy across
  multiple frequencies, triggering more of the ear's frequency response range
- **RMS power**: Square waves maintain maximum amplitude continuously (RMS =
  1.0), while other waveforms spend time at intermediate values
- **Psychoacoustics**: The human ear is more sensitive to sounds with energy
  spread across the frequency spectrum

The sine wave sounds quietest despite decent RMS power (~0.707) because it lacks
the harmonic richness that makes other waveforms perceptually louder.

## Mixing

This application does not provide level control or mixing. If you need to
balance voices playing simultaneously, you'll need to do it downstream.
https://github.com/odaacabeef/stems is a live monitoring and multi-track
recorder being developed as a companion for this.
