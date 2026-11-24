//! Little chiptune sound effects, synthesised on the fly (no sound files).
//!
//! The audio device is only opened the first time a sound plays, so with
//! sounds switched off Tabularium never touches the sound card.

use crate::themes::ThemeId;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SizedSample};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Sfx {
    /// Moving between pages.
    Click,
    /// Choosing something (a theme, a random pick).
    Select,
    /// A long job finished.
    Done,
    /// Something went wrong.
    Error,
    /// The start-up screen's jingle.
    Boot,
}

#[derive(Clone, Copy)]
enum Wave {
    /// Pulse wave with this duty cycle, like the NES and Game Boy.
    Pulse(f32),
    Triangle,
    /// Sawtooth, like the C64's SID chip.
    Saw,
    Sine,
}

#[derive(Clone, Copy)]
struct Tone {
    /// Hz; 0 is a rest.
    freq: f32,
    secs: f32,
    wave: Wave,
    vol: f32,
}

struct Out {
    _stream: cpal::Stream,
    queue: Arc<Mutex<VecDeque<Tone>>>,
}

#[derive(Default)]
pub struct Sound {
    out: Option<Out>,
    failed: bool,
}

fn voice(theme: ThemeId) -> Wave {
    match theme {
        ThemeId::Nes | ThemeId::Famicom => Wave::Pulse(0.25),
        ThemeId::GameBoy | ThemeId::AtomicPurple | ThemeId::ArcadeCrt => Wave::Pulse(0.5),
        ThemeId::VirtualBoy => Wave::Pulse(0.125),
        ThemeId::Commodore64 | ThemeId::Cyberpunk => Wave::Saw,
        ThemeId::GameCube => Wave::Triangle,
        ThemeId::Standard => Wave::Sine,
    }
}

/// (frequency, seconds) pairs for each effect.
fn notes(sfx: Sfx, theme: ThemeId) -> Vec<(f32, f32)> {
    match sfx {
        Sfx::Click => vec![(880.0, 0.035)],
        Sfx::Select => vec![(660.0, 0.05), (990.0, 0.08)],
        Sfx::Done => vec![(523.0, 0.07), (659.0, 0.07), (784.0, 0.07), (1047.0, 0.18)],
        Sfx::Error => vec![(247.0, 0.11), (175.0, 0.22)],
        Sfx::Boot => match theme {
            ThemeId::GameBoy | ThemeId::AtomicPurple => {
                vec![(0.0, 0.05), (1319.0, 0.07), (2637.0, 0.4)]
            }
            ThemeId::Nes | ThemeId::Famicom => vec![
                (523.0, 0.08),
                (659.0, 0.08),
                (784.0, 0.08),
                (1047.0, 0.08),
                (784.0, 0.08),
                (1047.0, 0.25),
            ],
            ThemeId::Commodore64 => vec![(196.0, 0.1), (262.0, 0.1), (392.0, 0.1), (523.0, 0.3)],
            ThemeId::ArcadeCrt => vec![(784.0, 0.05), (1175.0, 0.05), (1568.0, 0.22)],
            ThemeId::VirtualBoy => vec![(196.0, 0.12), (294.0, 0.12), (392.0, 0.3)],
            ThemeId::GameCube => vec![
                (523.0, 0.12),
                (659.0, 0.12),
                (784.0, 0.12),
                (988.0, 0.12),
                (1047.0, 0.4),
            ],
            ThemeId::Cyberpunk => vec![
                (110.0, 0.08),
                (220.0, 0.08),
                (440.0, 0.08),
                (880.0, 0.08),
                (0.0, 0.05),
                (880.0, 0.2),
            ],
            ThemeId::Standard => vec![(660.0, 0.08), (880.0, 0.15)],
        },
    }
}

impl Sound {
    /// Play an effect in the current theme's voice. `volume` is 0..=1.
    pub fn play(&mut self, sfx: Sfx, theme: ThemeId, volume: f32) {
        if volume <= 0.0 {
            return;
        }
        let Some(out) = self.output() else { return };
        let wave = voice(theme);
        // Pulse and saw waves are much louder than sines at the same level.
        let level = match wave {
            Wave::Sine | Wave::Triangle => 0.35,
            _ => 0.16,
        } * volume.clamp(0.0, 1.0);
        let mut q = out.queue.lock().unwrap();
        // Clicks shouldn't pile up behind each other.
        if sfx == Sfx::Click && q.len() > 2 {
            return;
        }
        q.extend(notes(sfx, theme).into_iter().map(|(freq, secs)| Tone {
            freq,
            secs,
            wave,
            vol: level,
        }));
    }

    fn output(&mut self) -> Option<&Out> {
        if self.out.is_none() && !self.failed {
            match open() {
                Some(o) => self.out = Some(o),
                None => self.failed = true,
            }
        }
        self.out.as_ref()
    }
}

fn open() -> Option<Out> {
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => build::<f32>(&device, config.into(), queue.clone()),
        cpal::SampleFormat::I16 => build::<i16>(&device, config.into(), queue.clone()),
        cpal::SampleFormat::U16 => build::<u16>(&device, config.into(), queue.clone()),
        cpal::SampleFormat::I32 => build::<i32>(&device, config.into(), queue.clone()),
        _ => None,
    }?;
    stream.play().ok()?;
    Some(Out {
        _stream: stream,
        queue,
    })
}

fn build<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    queue: Arc<Mutex<VecDeque<Tone>>>,
) -> Option<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let rate = config.sample_rate as f32;
    let channels = config.channels as usize;
    let mut synth = Synth {
        rate,
        current: None,
        pos: 0,
        len: 0,
        phase: 0.0,
    };
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_mut(channels) {
                    let v = T::from_sample(synth.next(&queue));
                    for s in frame.iter_mut() {
                        *s = v;
                    }
                }
            },
            |_| {},
            None,
        )
        .ok()
}

struct Synth {
    rate: f32,
    current: Option<Tone>,
    pos: usize,
    len: usize,
    phase: f32,
}

impl Synth {
    fn next(&mut self, queue: &Mutex<VecDeque<Tone>>) -> f32 {
        if self.current.is_none() || self.pos >= self.len {
            // Never block the audio thread waiting for the UI.
            let next = queue.try_lock().ok().and_then(|mut q| q.pop_front());
            self.current = next;
            self.pos = 0;
            self.len = next.map(|t| (t.secs * self.rate) as usize).unwrap_or(0);
        }
        let Some(t) = self.current else {
            return f32::EQUILIBRIUM;
        };
        self.pos += 1;
        if t.freq <= 0.0 {
            return 0.0;
        }
        self.phase = (self.phase + t.freq / self.rate).fract();
        let p = self.phase;
        let raw = match t.wave {
            Wave::Pulse(duty) => {
                if p < duty {
                    1.0
                } else {
                    -1.0
                }
            }
            Wave::Triangle => 4.0 * (p - 0.5).abs() - 1.0,
            Wave::Saw => 2.0 * p - 1.0,
            Wave::Sine => (p * std::f32::consts::TAU).sin(),
        };
        // Short fade in and out so notes don't click.
        let fade = (0.004 * self.rate).max(1.0);
        let left = (self.len - self.pos) as f32;
        let env = (self.pos as f32 / fade).min(1.0) * (left / (fade * 3.0)).min(1.0);
        raw * t.vol * env
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn synth_plays_queued_notes_then_goes_quiet() {
        let rate = 8000.0;
        let queue = Mutex::new(VecDeque::new());
        for (freq, secs) in notes(Sfx::Boot, ThemeId::Nes) {
            queue.lock().unwrap().push_back(Tone {
                freq,
                secs,
                wave: voice(ThemeId::Nes),
                vol: 0.2,
            });
        }
        let total: f32 = notes(Sfx::Boot, ThemeId::Nes).iter().map(|n| n.1).sum();
        let mut synth = Synth {
            rate,
            current: None,
            pos: 0,
            len: 0,
            phase: 0.0,
        };
        let samples: Vec<f32> = (0..(total * rate) as usize + 800)
            .map(|_| synth.next(&queue))
            .collect();
        let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.15 && peak <= 0.2, "peak {peak}");
        assert!(samples[samples.len() - 100..].iter().all(|s| *s == 0.0));
        assert!(queue.lock().unwrap().is_empty());
    }
}
