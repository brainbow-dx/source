//! Short procedurally-synthesized sound effects: a jump arpeggio, an attack slash, a light and
//! heavy hit crunch plus a hurt cue, and a death pop. No audio asset files, no external
//! synthesizer or MIDI dependency: each clip is a handful of milliseconds of raw PCM built from a
//! couple of oscillators and a decay envelope, encoded as a minimal WAV in memory, and handed to
//! `bevy_audio` (`wav`/`bevy_audio` features enabled on `bevy` in `Cargo.toml` for exactly this)
//! the same way a loaded asset file would be.

use std::f32::consts::TAU;

use bevy::asset::Assets;
use bevy::asset::Handle;
use bevy::audio::AudioPlayer;
use bevy::audio::AudioSource;
use bevy::audio::PlaybackSettings;
use bevy::ecs::resource::Resource;
use bevy::ecs::system::Commands;
use bevy::ecs::system::ResMut;

/// Low enough to keep every clip a few hundred bytes to a couple KB, high enough that a short
/// blip doesn't sound gritty. These are simple tones and noise bursts, not music, so there's
/// nothing to gain from a higher rate here.
const SFX_SAMPLE_RATE: u32 = 22_050;

/// Handles to every clip this example plays, synthesized once at startup and reused for every
/// play. `AudioPlayer` clones the handle cheaply per spawn; there's no need to resynthesize or
/// reload anything on each jump/attack/hit/death.
#[derive(Resource)]
pub struct MarioSfx {
    pub jump: Handle<AudioSource>,
    pub attack: Handle<AudioSource>,
    pub hit_crunch: Handle<AudioSource>,
    /// A heavier crunch for a hit landed while holding a trigger (see `MARIO_HEAVY_HIT_BUTTONS`
    /// in `physics.rs`) — this is `synth_hit_crunch`, the same synth briefly used for `jump`
    /// itself, moved here per direct user request once a real jump sound existed to replace it.
    pub heavy_hit: Handle<AudioSource>,
    pub hit_hurt: Handle<AudioSource>,
    pub death: Handle<AudioSource>,
}

pub fn setup_mario_sfx(mut commands: Commands, mut audio_sources: ResMut<Assets<AudioSource>>) {
    let mut add = |samples: Vec<i16>| audio_sources.add(AudioSource { bytes: wav_bytes(SFX_SAMPLE_RATE, &samples).into() });

    commands.insert_resource(MarioSfx {
        jump: add(synth_jump_sequence()),
        attack: add(synth_attack_slash()),
        hit_crunch: add(synth_jump_blip()),
        heavy_hit: add(synth_hit_crunch()),
        hit_hurt: add(synth_hit_hurt()),
        death: add(synth_death_pop()),
    });
}

/// Spawns a one-shot player for `handle`. `PlaybackSettings::DESPAWN` means the entity cleans
/// itself up once the clip finishes, so callers never need to track or despawn these themselves.
pub fn play(commands: &mut Commands, handle: &Handle<AudioSource>) {
    commands.spawn((AudioPlayer::new(handle.clone()), PlaybackSettings::DESPAWN));
}

/// A quick upward sine sweep. No longer used for the jump itself (see `synth_jump_sequence`
/// below), but still a good short, light "blip" — now the light hit's own crunch.
fn synth_jump_blip() -> Vec<i16> {
    synth(0.09, |t, duration| {
        let progress = t / duration;
        let freq = 420.0 + progress * 480.0;
        sine(freq, t) * linear_decay(progress)
    })
}

/// A quick ascending four-note square-wave arpeggio, closer to a real Mario jump cue than a
/// single sliding tone — each note gets its own short percussive decay rather than one smooth
/// sweep, so it reads as a "doot-doot-doot-doot" run.
fn synth_jump_sequence() -> Vec<i16> {
    const NOTES_HZ: [f32; 4] = [523.25, 659.25, 783.99, 1046.50]; // C5, E5, G5, C6
    const NOTE_DURATION: f32 = 0.045;

    let mut samples = Vec::new();
    for &freq in &NOTES_HZ {
        samples.extend(synth(NOTE_DURATION, move |t, duration| {
            let progress = t / duration;
            square(freq, t) * exponential_decay(progress, 4.0)
        }));
    }
    samples
}

/// A fast downward sweep with a touch of noise mixed in, reading as a whoosh rather than a tone.
fn synth_attack_slash() -> Vec<i16> {
    let mut noise = Xorshift32::new(0xA77A_C4);
    synth(0.08, move |t, duration| {
        let progress = t / duration;
        let freq = 1300.0 - progress * 900.0;
        let tone = sine(freq, t);
        let hiss = noise.next_signed();
        (tone * 0.7 + hiss * 0.3) * linear_decay(progress)
    })
}

/// A short burst of low-passed noise: a thud rather than a tone, for a landed heavy hit's impact
/// (see `MarioSfx::heavy_hit`). The one-pole lowpass is deliberately crude (a running average of
/// consecutive samples) — enough to round the noise off into a crunch instead of a hiss, nothing
/// fancier is needed.
fn synth_hit_crunch() -> Vec<i16> {
    let mut noise = Xorshift32::new(0xC70C_11);
    let mut previous = 0.0;
    synth(0.07, move |t, duration| {
        let progress = t / duration;
        let raw = noise.next_signed();
        let filtered = previous * 0.6 + raw * 0.4;
        previous = filtered;
        filtered * exponential_decay(progress, 6.0)
    })
}

/// A short descending tone, the "hurt" half of a landed hit, played alongside the crunch above.
fn synth_hit_hurt() -> Vec<i16> {
    synth(0.16, |t, duration| {
        let progress = t / duration;
        let freq = 520.0 - progress * 320.0;
        square(freq, t) * 0.5 * exponential_decay(progress, 3.0)
    })
}

/// A longer, heavier noise burst for a death: the same low-pass approach as the hit crunch, but
/// longer, more aggressively filtered (less hiss, more thud), and layered with a low sine "sub"
/// dropping in pitch underneath the noise — a boom, not just a bigger crunch.
fn synth_death_pop() -> Vec<i16> {
    let mut noise = Xorshift32::new(0xDEAD_B0);
    let mut previous = 0.0;
    synth(0.5, move |t, duration| {
        let progress = t / duration;
        let raw = noise.next_signed();
        let filtered = previous * 0.85 + raw * 0.15;
        previous = filtered;
        let sub_freq = 90.0 - progress * 50.0;
        let sub = sine(sub_freq, t);
        (filtered * 0.6 + sub * 0.6) * exponential_decay(progress, 2.5)
    })
}

fn sine(freq_hz: f32, t: f32) -> f32 {
    (t * freq_hz * TAU).sin()
}

fn square(freq_hz: f32, t: f32) -> f32 {
    if sine(freq_hz, t) >= 0.0 { 1.0 } else { -1.0 }
}

fn linear_decay(progress: f32) -> f32 {
    (1.0 - progress).clamp(0.0, 1.0)
}

/// Steeper than `linear_decay`: most of the clip's energy is in its first fraction, which reads
/// as a percussive hit/pop rather than a tone fading out evenly.
fn exponential_decay(progress: f32, sharpness: f32) -> f32 {
    (-progress * sharpness).exp()
}

/// Calls `sample_at(t, duration)` once per sample across `duration` seconds at `SFX_SAMPLE_RATE`,
/// converting its -1.0..=1.0 output to 16-bit PCM.
fn synth(duration: f32, mut sample_at: impl FnMut(f32, f32) -> f32) -> Vec<i16> {
    let sample_count = (duration * SFX_SAMPLE_RATE as f32) as usize;
    (0..sample_count)
        .map(|index| {
            let t = index as f32 / SFX_SAMPLE_RATE as f32;
            (sample_at(t, duration).clamp(-1.0, 1.0) * i16::MAX as f32) as i16
        })
        .collect()
}

/// A tiny xorshift PRNG, deterministic per seed and dependency-free — plenty for generating a
/// short noise burst; nothing here needs cryptographic or even statistical quality randomness.
struct Xorshift32(u32);

impl Xorshift32 {
    fn new(seed: u32) -> Self {
        Xorshift32(seed | 1)
    }

    fn next_signed(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// A minimal mono 16-bit PCM WAV: just enough header for `rodio`'s decoder to accept it.
fn wav_bytes(sample_rate: u32, samples: &[i16]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let byte_rate = sample_rate * 2;

    let mut bytes = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&sample_rate.to_le_bytes());
    bytes.extend_from_slice(&byte_rate.to_le_bytes());
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align (mono * 16-bit)
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for &sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    bytes
}
