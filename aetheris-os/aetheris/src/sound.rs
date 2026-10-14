//! Buzzer sound design: system sounds as (start Hz, end Hz, ms) steps. A step with f0 != f1 glides.
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Step {
    pub f0: u16,
    pub f1: u16,
    pub ms: u16,
}

pub const fn t(f: u16, ms: u16) -> Step {
    Step { f0: f, f1: f, ms }
}
pub const fn g(f0: u16, f1: u16, ms: u16) -> Step {
    Step { f0, f1, ms }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Snd {
    Boot,
    BootBeep,
    Click,
    Accept,
    Error,
    Back,
    Notify,
    Alarm,
    Lock,
    Unlock,
    Connect,
    Disconnect,
    Warn,
    Shutdown,
    Ping,
    Select,
    Typing,
    Win,
    Lose,
}

pub fn steps(s: Snd) -> &'static [Step] {
    match s {
        Snd::Boot => const { &[
            g(120, 700, 220), t(0, 50),
            t(523, 110), t(659, 110), t(784, 110), t(1047, 220), t(0, 40),
            t(784, 70), t(1047, 70), t(1319, 70), t(1568, 120), t(0, 20),
            t(1568, 60), t(1319, 60), t(1568, 60), t(2093, 260),
            g(2093, 3136, 140),
        ] },
        Snd::BootBeep => const { &[t(1400, 25)] },
        Snd::Click => const { &[t(2400, 7)] },
        Snd::Typing => const { &[t(1900, 9)] },
        Snd::Select => const { &[t(1568, 18), t(2093, 22)] },
        Snd::Accept => const { &[t(880, 70), t(0, 15), t(1319, 120), t(1760, 140)] },
        Snd::Error => const { &[t(196, 160), t(0, 40), t(147, 260)] },
        Snd::Back => const { &[t(784, 40), t(523, 60)] },
        Snd::Notify => const { &[t(1047, 80), t(0, 40), t(1047, 80), t(1319, 180)] },
        Snd::Alarm => const { &[
            t(2093, 120), t(0, 60), t(2093, 120), t(0, 60), t(2093, 120), t(0, 260),
            t(2093, 120), t(0, 60), t(2093, 120), t(0, 60), t(2093, 120), t(0, 260),
        ] },
        Snd::Lock => const { &[t(784, 70), t(523, 120)] },
        Snd::Unlock => const { &[t(523, 70), t(784, 70), t(1047, 130)] },
        Snd::Connect => const { &[t(784, 70), t(988, 70), t(1319, 160)] },
        Snd::Disconnect => const { &[t(988, 80), t(659, 160)] },
        Snd::Warn => const { &[t(440, 100), t(0, 60), t(440, 100)] },
        Snd::Shutdown => const { &[t(1047, 90), t(784, 90), t(523, 90), g(523, 130, 300)] },
        Snd::Ping => const { &[t(1760, 25)] },
        Snd::Win => const { &[t(523, 80), t(659, 80), t(784, 80), t(1047, 200)] },
        Snd::Lose => const { &[g(600, 150, 400)] },
    }
}

/// Convert a melody (MIDI, 16th-note length) at `bpm` into buzzer steps with short articulation gaps.
pub fn melody(notes: &[(u8, u8)], bpm: u16) -> Vec<Step> {
    let sixteenth = 15_000u32 / bpm.max(30) as u32;
    let mut v = Vec::with_capacity(notes.len() * 2);
    for &(n, len) in notes {
        let total = sixteenth * len as u32;
        if n == 0 {
            v.push(t(0, total as u16));
        } else {
            let gap = (total / 8).clamp(8, 40);
            v.push(t(crate::notes::freq(n as i32) as u16, (total - gap) as u16));
            v.push(t(0, gap as u16));
        }
    }
    v
}

/// International Morse for A-Z, 0-9 (dot = 1, dash = 3).
pub fn morse_code(c: char) -> &'static str {
    match c.to_ascii_uppercase() {
        'A' => ".-", 'B' => "-...", 'C' => "-.-.", 'D' => "-..", 'E' => ".", 'F' => "..-.",
        'G' => "--.", 'H' => "....", 'I' => "..", 'J' => ".---", 'K' => "-.-", 'L' => ".-..",
        'M' => "--", 'N' => "-.", 'O' => "---", 'P' => ".--.", 'Q' => "--.-", 'R' => ".-.",
        'S' => "...", 'T' => "-", 'U' => "..-", 'V' => "...-", 'W' => ".--", 'X' => "-..-",
        'Y' => "-.--", 'Z' => "--..",
        '0' => "-----", '1' => ".----", '2' => "..---", '3' => "...--", '4' => "....-",
        '5' => ".....", '6' => "-....", '7' => "--...", '8' => "---..", '9' => "----.",
        '.' => ".-.-.-", ',' => "--..--", '?' => "..--..", '!' => "-.-.--", '@' => ".--.-.",
        _ => "",
    }
}

pub fn morse_steps(text: &str, wpm: u16, freq: u16) -> Vec<Step> {
    let unit = (1200 / wpm.max(5)) as u16;
    let mut v = Vec::new();
    for w in text.split(' ') {
        for ch in w.chars() {
            let code = morse_code(ch);
            for s in code.chars() {
                v.push(t(freq, if s == '.' { unit } else { unit * 3 }));
                v.push(t(0, unit));
            }
            if !code.is_empty() {
                v.push(t(0, unit * 2));
            }
        }
        v.push(t(0, unit * 4));
    }
    v
}
