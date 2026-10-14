//! Passive-buzzer sound engine on LEDC PWM. A task plays sequences, glides and sustained tones,
//! and a new command always pre-empts the one being played.
use super::shared::{SOUND, SoundCmd, VOLUME};
use crate::sound::Step;
use core::sync::atomic::Ordering;
use embassy_futures::select::{Either, select};
use embassy_time::{Duration, Timer};
use esp_hal::gpio::{AnyPin, DriveMode};
use esp_hal::ledc::channel::{self, ChannelIFace};
use esp_hal::ledc::timer::{self, TimerIFace};
use esp_hal::ledc::{LSGlobalClkSource, Ledc, LowSpeed};
use esp_hal::peripherals::LEDC;
use esp_hal::time::Rate;

/// Duty cycle (%) per volume step 0..=5: square-wave duty controls loudness on a piezo.
const DUTY: [u8; 6] = [0, 4, 10, 22, 36, 50];

struct Player<'a> {
    ledc: &'a Ledc<'static>,
    ch: channel::Channel<'a, LowSpeed>,
    cur: u16,
}

impl<'a> Player<'a> {
    fn on(&mut self, f: u16) {
        let duty = DUTY[(VOLUME.load(Ordering::Relaxed) as usize).min(5)];
        if f < 60 || duty == 0 {
            let _ = self.ch.set_duty(0);
            self.cur = 0;
            return;
        }
        let f = f.min(4800);
        if f != self.cur {
            let mut t = self.ledc.timer::<LowSpeed>(timer::Number::Timer0);
            let _ = t.configure(timer::config::Config {
                duty: timer::config::Duty::Duty14Bit,
                clock_source: timer::LSClockSource::APBClk,
                frequency: Rate::from_hz(f as u32),
            });
            self.cur = f;
        }
        let _ = self.ch.set_duty(duty);
    }
    fn off(&mut self) {
        let _ = self.ch.set_duty(0);
        self.cur = 0;
    }
}

/// Plays steps until finished; returns a command that arrived meanwhile (to be handled next).
async fn play<I: Iterator<Item = Step>>(p: &mut Player<'_>, steps: I) -> Option<SoundCmd> {
    for s in steps {
        if s.f0 == s.f1 {
            if s.f0 == 0 {
                p.off();
            } else {
                p.on(s.f0);
            }
            if let Either::Second(c) = select(Timer::after(Duration::from_millis(s.ms as u64)), SOUND.receive()).await {
                p.off();
                return Some(c);
            }
        } else {
            let n = (s.ms / 10).max(1) as i32;
            for i in 0..n {
                let f = s.f0 as i32 + (s.f1 as i32 - s.f0 as i32) * i / n;
                p.on(f as u16);
                if let Either::Second(c) = select(Timer::after(Duration::from_millis(10)), SOUND.receive()).await {
                    p.off();
                    return Some(c);
                }
            }
        }
    }
    p.off();
    None
}

#[embassy_executor::task]
pub async fn buzzer_task(ledc_per: LEDC<'static>, pin: AnyPin<'static>) {
    let mut ledc = Ledc::new(ledc_per);
    ledc.set_global_slow_clock(LSGlobalClkSource::APBClk);
    let mut t0 = ledc.timer::<LowSpeed>(timer::Number::Timer0);
    let _ = t0.configure(timer::config::Config {
        duty: timer::config::Duty::Duty14Bit,
        clock_source: timer::LSClockSource::APBClk,
        frequency: Rate::from_hz(1000),
    });
    let mut ch = ledc.channel::<LowSpeed>(channel::Number::Channel0, pin);
    let _ = ch.configure(channel::config::Config { timer: &t0, duty_pct: 0, drive_mode: DriveMode::PushPull });
    let mut p = Player { ledc: &ledc, ch, cur: 0 };
    let mut pending: Option<SoundCmd> = None;
    loop {
        let cmd = match pending.take() {
            Some(c) => c,
            None => SOUND.receive().await,
        };
        match cmd {
            SoundCmd::Tone(f) => p.on(f),
            SoundCmd::Silence => p.off(),
            SoundCmd::Seq(s) => pending = play(&mut p, s.iter().copied()).await,
            SoundCmd::Owned(v) => pending = play(&mut p, v.into_iter()).await,
        }
    }
}
