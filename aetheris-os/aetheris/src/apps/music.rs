//! Sound apps: piano (notes, sharps/flats, chords), music box, metronome, tone lab, morse.
use super::{App, Flow, aux_panel};
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::input::{Key, KeyEvent, Mode, TextEntry};
use crate::notes;
use crate::sound::{self, Snd};
use crate::sys::Sys;
use crate::trig::*;
use crate::tunes::TUNES;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

// ───────────────────────────── Piano ─────────────────────────────
/// Phone key -> semitone above C (reading order of the 4x3 digit block).
fn semitone(k: Key) -> Option<i32> {
    Some(match k {
        Key::K1 => 0, Key::K2 => 1, Key::K3 => 2,
        Key::K4 => 3, Key::K5 => 4, Key::K6 => 5,
        Key::K7 => 6, Key::K8 => 7, Key::K9 => 8,
        Key::Star => 9, Key::K0 => 10, Key::Hash => 11,
        _ => return None,
    })
}
const PHONE_LABEL: [&str; 12] = ["1", "2", "3", "4", "5", "6", "7", "8", "9", "*", "0", "#"];

pub struct Piano {
    chord: bool,
    oct: i32,
    ch: usize,
    held: Option<Key>,
    root: i32,
    arp: usize,
    next_arp: u64,
    pub lit: u16,
}

impl Piano {
    pub fn new() -> Self {
        Self { chord: false, oct: 4, ch: 0, held: None, root: 60, arp: 0, next_arp: 0, lit: 0 }
    }
    fn chord_notes(&self) -> Vec<i32> {
        notes::CHORDS[self.ch].iv.iter().map(|&i| self.root + i as i32).collect()
    }
    fn set_lit(&mut self) {
        self.lit = 0;
        if self.held.is_some() {
            if self.chord {
                for n in self.chord_notes() {
                    self.lit |= 1 << (n.rem_euclid(12));
                }
            } else {
                self.lit |= 1 << (self.root.rem_euclid(12));
            }
        }
    }
}

impl App for Piano {
    fn owns_star(&self) -> bool {
        true
    }
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if let Some(s) = semitone(ev.key) {
            if ev.press() {
                self.root = 12 * (self.oct + 1) + s;
                self.held = Some(ev.key);
                self.arp = 0;
                self.next_arp = sys.now + 70;
                let first = if self.chord { self.chord_notes()[0] } else { self.root };
                sys.tone(notes::freq(first) as u16);
                sys.led_pulse(80);
            } else if ev.release() && self.held == Some(ev.key) {
                self.held = None;
                sys.silence();
            }
            self.set_lit();
            return Flow::Stay;
        }
        if !ev.press() {
            return Flow::Stay;
        }
        match ev.key {
            Key::A => {
                if self.chord {
                    self.ch = (self.ch + 1) % notes::CHORDS.len();
                } else {
                    self.oct = (self.oct + 1).min(7);
                }
                sys.snd(Snd::Click);
            }
            Key::B => {
                if self.chord {
                    self.ch = (self.ch + notes::CHORDS.len() - 1) % notes::CHORDS.len();
                } else {
                    self.oct = (self.oct - 1).max(1);
                }
                sys.snd(Snd::Click);
            }
            Key::C => {
                self.chord = !self.chord;
                self.held = None;
                sys.silence();
                sys.snd(Snd::Select);
            }
            Key::D => return Flow::Exit,
            _ => {}
        }
        self.set_lit();
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.chord && self.held.is_some() && sys.now >= self.next_arp {
            let n = self.chord_notes();
            self.arp = (self.arp + 1) % n.len();
            sys.tone(notes::freq(n[self.arp]) as u16);
            self.next_arp = sys.now + 55;
        }
    }
    fn on_exit(&mut self, sys: &mut Sys) {
        sys.silence();
    }
    fn keep_awake(&self) -> bool {
        self.held.is_some()
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, if self.chord { "CHORDS" } else { "PIANO" });
        gfx::text_r(a, 125, 3, &format!("Oct {}", self.oct), F::M6x10, true);
        if self.chord {
            let nn = self.chord_notes();
            let rootn = notes::name(self.root);
            gfx::text(a, 4, 19, &format!("{} {}", rootn, notes::CHORDS[self.ch].name), F::XL9x15, true);
            let mut s = String::new();
            for n in nn.iter() {
                s.push_str(notes::name(*n));
                s.push(' ');
            }
            gfx::text(a, 4, 37, &s, F::M6x10, true);
            gfx::hint(a, "key=root A/B:type C:note");
        } else {
            gfx::text(a, 4, 19, &format!("{}{}", notes::name(self.root), notes::octave(self.root)), F::H10x20, true);
            let x = notes::freq_x100(self.root);
            gfx::text(a, 4, 41, &format!("{}.{:02} Hz", x / 100, x % 100), F::M6x10, true);
            gfx::hint(a, "A/B:octave C:chords D:x");
        }
        // one-octave keyboard on the second screen
        let whites = [0, 2, 4, 5, 7, 9, 11];
        for (i, &s) in whites.iter().enumerate() {
            let x = 4 + i as i32 * 17;
            let lit = self.lit & (1 << s) != 0;
            if lit {
                b.fill_rect(x, 8, 17, 50, true);
            } else {
                b.rect(x, 8, 17, 50, true);
            }
            gfx::text_c(b, x + 8, 49, PHONE_LABEL[s as usize], F::S5x8, !lit);
        }
        for (i, s) in [(0, 1), (1, 3), (3, 6), (4, 8), (5, 10)] {
            let x = 4 + (i + 1) * 17 - 5;
            let lit = self.lit & (1 << s) != 0;
            b.fill_rect(x, 8, 10, 29, false);
            if lit {
                b.rect(x, 8, 10, 29, true);
                b.fill_rect(x + 3, 26, 4, 4, true);
            } else {
                b.fill_rect(x, 8, 10, 29, true);
                gfx::text_c(b, x + 5, 27, PHONE_LABEL[s as usize], F::S5x8, false);
            }
        }
        gfx::text(b, 4, 0, "C  D  E  F  G  A  B", F::S5x8, false);
        b.fill_rect(0, 0, 128, 7, false);
        gfx::text(b, 2, 0, &format!("OCTAVE {}  C{}={}Hz", self.oct, self.oct, notes::freq(12 * (self.oct + 1))), F::S5x8, true);
        let _ = F::S4x6;
    }
}

// ───────────────────────────── Music box ─────────────────────────────
pub struct MusicBox {
    sel: usize,
    playing: Option<(u64, usize)>,
    total: u64,
    roll: Vec<(u32, u32, u8)>,
}

impl MusicBox {
    pub fn new() -> Self {
        let mut m = Self { sel: 0, playing: None, total: 0, roll: Vec::new() };
        m.build_roll();
        m
    }
    fn build_roll(&mut self) {
        let t = &TUNES[self.sel];
        let sixteenth = 15_000u32 / t.bpm as u32;
        let mut pos = 0u32;
        self.roll.clear();
        for &(n, len) in t.notes {
            let dur = sixteenth * len as u32;
            if n != 0 {
                self.roll.push((pos, dur, n));
            }
            pos += dur;
        }
        self.total = pos as u64;
    }
    fn start(&mut self, sys: &mut Sys) {
        let t = &TUNES[self.sel];
        let steps = sound::melody(t.notes, t.bpm);
        self.build_roll();
        sys.play(steps);
        self.playing = Some((sys.now, self.sel));
    }
}

impl App for MusicBox {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.down() {
            return Flow::Stay;
        }
        use crate::input::Nav::*;
        match ev.key.nav() {
            Some(Up) | Some(Left) => {
                self.sel = (self.sel + TUNES.len() - 1) % TUNES.len();
                self.build_roll();
                self.playing = None;
                sys.silence();
                sys.snd(Snd::Click);
            }
            Some(Down) | Some(Right) => {
                self.sel = (self.sel + 1) % TUNES.len();
                self.build_roll();
                self.playing = None;
                sys.silence();
                sys.snd(Snd::Click);
            }
            Some(Ok) if ev.press() => {
                if self.playing.is_some() {
                    self.playing = None;
                    sys.silence();
                } else {
                    self.start(sys);
                }
            }
            Some(Back) if ev.press() => {
                sys.silence();
                return Flow::Exit;
            }
            _ => {}
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if let Some((t0, _)) = self.playing
            && sys.now - t0 > self.total + 100
        {
            self.playing = None;
        }
    }
    fn on_exit(&mut self, sys: &mut Sys) {
        sys.silence();
    }
    fn keep_awake(&self) -> bool {
        self.playing.is_some()
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "MUSIC BOX");
        let t = &TUNES[self.sel];
        gfx::text(a, 4, 19, &gfx::clip(t.name, 18), F::L7x13, true);
        gfx::text(a, 4, 34, &format!("{}/{}  {} bpm", self.sel + 1, TUNES.len(), t.bpm), F::M6x10, true);
        let el = self.playing.map(|(t0, _)| (sys.now - t0).min(self.total)).unwrap_or(0);
        gfx::bar(a, 4, 45, 120, 8, el as i32, self.total.max(1) as i32);
        gfx::hint(a, "up/dn:song #:play *:back");
        aux_panel(b, "PIANO ROLL", &[]);
        let pos = el as i32;
        for &(s, d, n) in self.roll.iter() {
            let x0 = 24 + (s as i32 - pos) / 25;
            let x1 = x0 + (d as i32 / 25).max(2) - 1;
            if x1 < 0 || x0 > 127 {
                continue;
            }
            let y = (62 - (n as i32 - 48) * 3 / 2).clamp(14, 60);
            let lit = self.playing.is_some() && x0 <= 24 && x1 >= 24;
            b.fill_rect(x0.max(0), y, (x1 - x0.max(0) + 1).max(1), if lit { 4 } else { 3 }, true);
        }
        b.vline(24, 13, 50, true);
    }
}

// ───────────────────────────── Metronome ─────────────────────────────
pub struct Metronome {
    bpm: i32,
    sig: usize,
    run: bool,
    beat: usize,
    next: u64,
    t0: u64,
}
const SIGS: [(&str, usize); 4] = [("2/4", 2), ("3/4", 3), ("4/4", 4), ("6/8", 6)];
impl Metronome {
    pub fn new() -> Self {
        Self { bpm: 100, sig: 2, run: false, beat: 0, next: 0, t0: 0 }
    }
}
impl App for Metronome {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.down() {
            return Flow::Stay;
        }
        use crate::input::Nav::*;
        match ev.key.nav() {
            Some(Right) => self.bpm = (self.bpm + 1).min(240),
            Some(Left) => self.bpm = (self.bpm - 1).max(30),
            Some(Up) => self.bpm = (self.bpm + 5).min(240),
            Some(Down) => self.bpm = (self.bpm - 5).max(30),
            Some(Ok) if ev.press() => {
                self.run = !self.run;
                self.beat = 0;
                self.next = sys.now;
                self.t0 = sys.now;
            }
            Some(Back) if ev.press() => return Flow::Exit,
            _ => {}
        }
        if ev.press() && ev.key == Key::C {
            // C is Left in nav; use K0 to change signature instead (handled below)
        }
        if ev.press() && ev.key == Key::K0 {
            self.sig = (self.sig + 1) % SIGS.len();
            self.beat = 0;
            sys.snd(Snd::Select);
        }
        if ev.press() && ev.key == Key::K1 {
            self.bpm = 60;
        }
        if ev.press() && ev.key == Key::K3 {
            self.bpm = 120;
        }
        if ev.press() && ev.key == Key::K7 {
            self.bpm = 90;
        }
        if ev.press() && ev.key == Key::K9 {
            self.bpm = 160;
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.run && sys.now >= self.next {
            let accent = self.beat == 0;
            sys.play(alloc::vec![sound::t(if accent { 1568 } else { 1047 }, if accent { 35 } else { 22 })]);
            sys.led_pulse(70);
            self.beat = (self.beat + 1) % SIGS[self.sig].1;
            self.next += 60_000 / self.bpm as u64;
        }
    }
    fn keep_awake(&self) -> bool {
        self.run
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "METRONOME");
        gfx::text(a, 4, 19, &format!("{}", self.bpm), F::H10x20, true);
        gfx::text(a, 50, 24, "BPM", F::M6x10, true);
        gfx::text_r(a, 124, 24, SIGS[self.sig].0, F::XL9x15, true);
        gfx::hint(a, "<>:1 ^v:5 0:sig #:start");
        // pendulum
        let period = (60_000 / self.bpm as u64) * 2;
        let ph = if self.run { ((sys.now - self.t0) * 64 / period) as i32 } else { 0 };
        let swing = if self.run { sin64(ph) } else { 0 };
        let (px, py) = (64 + swing * 26 / 127, 56 - (cos64(ph).abs() * 0 / 127));
        let _ = py;
        b.line(64, 60, 64 + swing * 40 / 127, 14 + (127 - swing.abs()) * 6 / 127, true);
        b.line(63, 60, 63 + swing * 40 / 127, 14 + (127 - swing.abs()) * 6 / 127, true);
        b.disc(64 + swing * 40 / 127, 14 + (127 - swing.abs()) * 6 / 127, 4, true);
        let _ = px;
        b.fill_rect(56, 60, 16, 3, true);
        let n = SIGS[self.sig].1;
        for i in 0..n {
            let x = 6 + i as i32 * (116 / n as i32);
            let cur = self.run && (self.beat + n - 1) % n == i;
            if cur {
                b.fill_rect(x, 2, 8, 6, true);
            } else {
                b.rect(x, 2, 8, 6, true);
            }
        }
    }
}

// ───────────────────────────── Tone lab ─────────────────────────────
pub struct ToneLab {
    f: i32,
    on: bool,
    mode: u8,
    t: u64,
}
impl ToneLab {
    pub fn new() -> Self {
        Self { f: 440, on: false, mode: 0, t: 0 }
    }
}
impl App for ToneLab {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.down() {
            return Flow::Stay;
        }
        use crate::input::Nav::*;
        match ev.key.nav() {
            Some(Right) => self.f = (self.f + 10).min(5000),
            Some(Left) => self.f = (self.f - 10).max(50),
            Some(Up) => self.f = (self.f + 100).min(5000),
            Some(Down) => self.f = (self.f - 100).max(50),
            Some(Ok) if ev.press() => {
                self.on = !self.on;
                if self.on { sys.tone(self.f as u16) } else { sys.silence() }
            }
            Some(Back) if ev.press() => return Flow::Exit,
            _ => {}
        }
        if ev.press() && ev.key == Key::K0 {
            self.mode = (self.mode + 1) % 3;
            sys.snd(Snd::Select);
        }
        if ev.press() && ev.key == Key::K1 {
            self.f = 440;
        }
        if self.on && self.mode == 0 {
            sys.tone(self.f as u16);
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if !self.on || sys.now < self.t {
            return;
        }
        self.t = sys.now + 30;
        match self.mode {
            1 => {
                // sweep 200..4000
                self.f = 200 + ((sys.now / 4) % 3800) as i32;
                sys.tone(self.f as u16);
            }
            2 => {
                // siren
                let ph = (sys.now / 12) as i32;
                self.f = 900 + sin64(ph) * 5;
                sys.tone(self.f as u16);
            }
            _ => {}
        }
    }
    fn on_exit(&mut self, sys: &mut Sys) {
        sys.silence();
    }
    fn keep_awake(&self) -> bool {
        self.on
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "TONE LAB");
        gfx::text(a, 4, 19, &format!("{} Hz", self.f), F::H10x20, true);
        let m = ["MANUAL", "SWEEP", "SIREN"][self.mode as usize];
        gfx::text(a, 4, 41, m, F::M6x10, true);
        gfx::text_r(a, 124, 41, if self.on { "ON" } else { "off" }, F::M6x10, true);
        gfx::hint(a, "<>10 ^v100 0:mode #:on");
        // square wave whose period follows frequency
        aux_panel(b, "SQUARE WAVE", &[]);
        let per = (12_000 / self.f.max(1)).clamp(3, 60);
        let shift = if self.on { (sys.now / 20) as i32 % per } else { 0 };
        let mut prev = 0;
        for x in 0..128 {
            let hi = ((x + shift) % per) < per / 2;
            let y = if hi { 22 } else { 52 };
            b.set(x, y, true);
            if x > 0 && y != prev {
                b.vline(x, 22, 31, true);
            }
            prev = y;
        }
    }
}

// ───────────────────────────── Morse ─────────────────────────────
pub struct Morse {
    e: TextEntry,
    wpm: u16,
    sending: bool,
    on_iv: Vec<(u64, u64)>,
    t0: u64,
    end: u64,
}
impl Morse {
    pub fn new() -> Self {
        Self { e: TextEntry::new(40), wpm: 15, sending: false, on_iv: Vec::new(), t0: 0, end: 0 }
    }
}
impl App for Morse {
    fn owns_star(&self) -> bool {
        true
    }
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.press() {
            return Flow::Stay;
        }
        match ev.key {
            Key::Hash => {
                self.e.commit();
                let steps = sound::morse_steps(&self.e.text, self.wpm, 700);
                let mut t = 0u64;
                self.on_iv.clear();
                for s in steps.iter() {
                    if s.f0 != 0 {
                        self.on_iv.push((t, t + s.ms as u64));
                    }
                    t += s.ms as u64;
                }
                self.t0 = sys.now;
                self.end = t;
                self.sending = true;
                sys.play(steps);
            }
            Key::Star => {
                self.e.backspace();
                sys.snd(Snd::Typing);
            }
            Key::A => {
                self.e.cycle_mode();
                sys.snd(Snd::Click);
            }
            Key::B => {
                self.wpm = match self.wpm {
                    10 => 15,
                    15 => 20,
                    20 => 25,
                    _ => 10,
                };
                sys.snd(Snd::Select);
            }
            Key::C => self.e.clear(),
            Key::D => return Flow::Exit,
            k => {
                if self.e.key(k, sys.now) {
                    sys.snd(Snd::Typing);
                }
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        self.e.tick(sys.now);
        if self.sending {
            let el = sys.now - self.t0;
            if el > self.end {
                self.sending = false;
            } else if self.on_iv.iter().any(|&(s, e)| el >= s && el < e) {
                sys.led_pulse(60);
            }
        }
    }
    fn on_exit(&mut self, sys: &mut Sys) {
        sys.silence();
    }
    fn keep_awake(&self) -> bool {
        self.sending
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "MORSE");
        gfx::text_r(a, 125, 3, &format!("{}wpm", self.wpm), F::M6x10, true);
        let s = &self.e.text;
        let n = s.chars().count();
        let shown: String = s.chars().skip(n.saturating_sub(20)).collect();
        gfx::text(a, 4, 22, &shown, F::M6x10, true);
        if (sys.now / 450) % 2 == 0 {
            a.vline(4 + shown.chars().count() as i32 * 6, 21, 11, true);
        }
        gfx::text(a, 4, 38, self.e.mode.label(), F::M6x10, true);
        gfx::hint(a, if self.sending { "transmitting..." } else { "#:send A:abc B:wpm C:clr" });
        // morse symbols for the last characters on the second screen
        aux_panel(b, "MORSE CODE", &[]);
        let mut y = 16;
        let mut x = 2;
        let tail: String = s.chars().skip(n.saturating_sub(9)).collect();
        for ch in tail.chars() {
            let code = sound::morse_code(ch);
            let w = code.len() as i32 * 6 + 14;
            if x + w > 126 {
                x = 2;
                y += 13;
            }
            let mut buf = [0u8; 4];
            gfx::text(b, x, y, ch.to_ascii_uppercase().encode_utf8(&mut buf), F::S5x8, true);
            let mut cx = x + 8;
            for c in code.chars() {
                if c == '.' {
                    b.fill_rect(cx, y + 3, 2, 2, true);
                    cx += 4;
                } else {
                    b.fill_rect(cx, y + 3, 6, 2, true);
                    cx += 8;
                }
            }
            x = cx + 6;
        }
        let _ = Mode::Lower;
    }
}

// ───────────────────────────── Playlist builder ─────────────────────────────
const PL_MAX: usize = 16;

pub struct Playlist {
    /// Cursor in the library (all TUNES).
    lib: usize,
    /// Cursor in the saved playlist.
    pl_i: usize,
    /// false = browse library · true = edit playlist.
    show_pl: bool,
    /// Playing: (start_ms, playlist_index).
    playing: Option<(u64, usize)>,
    total: u64,
    loaded: bool,
}

impl Playlist {
    pub fn new() -> Self {
        Self { lib: 0, pl_i: 0, show_pl: false, playing: None, total: 0, loaded: false }
    }
    fn ensure(&mut self, sys: &mut Sys) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        sys.settings.playlist.retain(|&i| (i as usize) < TUNES.len());
        if sys.settings.playlist.len() > PL_MAX {
            sys.settings.playlist.truncate(PL_MAX);
        }
    }
    fn save(&mut self, sys: &mut Sys) {
        sys.save();
        sys.snd(Snd::Accept);
    }
    fn track_ms(idx: usize) -> u64 {
        let t = &TUNES[idx];
        let sixteenth = 15_000u32 / t.bpm.max(30) as u32;
        t.notes.iter().map(|&(_, len)| sixteenth * len as u32).sum::<u32>() as u64
    }
    fn start_at(&mut self, sys: &mut Sys, pi: usize) {
        let Some(&ti) = sys.settings.playlist.get(pi) else {
            sys.snd(Snd::Error);
            return;
        };
        let ti = ti as usize;
        if ti >= TUNES.len() {
            sys.snd(Snd::Error);
            return;
        }
        let t = &TUNES[ti];
        sys.play(sound::melody(t.notes, t.bpm));
        self.total = Self::track_ms(ti);
        self.playing = Some((sys.now, pi));
        self.pl_i = pi;
        self.show_pl = true;
    }
}

impl App for Playlist {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        self.ensure(sys);
        if !ev.down() {
            return Flow::Stay;
        }
        use crate::input::Nav::*;
        match ev.key.nav() {
            Some(Up) | Some(Left) => {
                if self.show_pl {
                    let n = sys.settings.playlist.len().max(1);
                    self.pl_i = (self.pl_i + n - 1) % n;
                } else {
                    self.lib = (self.lib + TUNES.len() - 1) % TUNES.len();
                }
                sys.snd(Snd::Click);
            }
            Some(Down) | Some(Right) => {
                if self.show_pl {
                    let n = sys.settings.playlist.len().max(1);
                    self.pl_i = (self.pl_i + 1) % n;
                } else {
                    self.lib = (self.lib + 1) % TUNES.len();
                }
                sys.snd(Snd::Click);
            }
            Some(Ok) if ev.press() => {
                if self.playing.is_some() {
                    self.playing = None;
                    sys.silence();
                    sys.snd(Snd::Back);
                } else if self.show_pl {
                    self.start_at(sys, self.pl_i.min(sys.settings.playlist.len().saturating_sub(1)));
                } else if sys.settings.playlist.len() < PL_MAX {
                    sys.settings.playlist.push(self.lib as u8);
                    self.save(sys);
                } else {
                    sys.snd(Snd::Error);
                }
            }
            Some(Back) if ev.press() => {
                sys.silence();
                return Flow::Exit;
            }
            _ => match ev.key {
                Key::A if ev.press() => {
                    self.show_pl = !self.show_pl;
                    sys.snd(Snd::Select);
                }
                Key::B if ev.press() => {
                    if self.show_pl {
                        if !sys.settings.playlist.is_empty() {
                            let i = self.pl_i.min(sys.settings.playlist.len() - 1);
                            sys.settings.playlist.remove(i);
                            if self.pl_i >= sys.settings.playlist.len() && self.pl_i > 0 {
                                self.pl_i -= 1;
                            }
                            self.save(sys);
                        } else {
                            sys.snd(Snd::Error);
                        }
                    } else if sys.settings.playlist.len() < PL_MAX {
                        sys.settings.playlist.push(self.lib as u8);
                        self.save(sys);
                    } else {
                        sys.snd(Snd::Error);
                    }
                }
                Key::C if ev.press() => {
                    sys.settings.playlist.clear();
                    self.pl_i = 0;
                    self.playing = None;
                    sys.silence();
                    self.save(sys);
                }
                _ => {}
            },
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        self.ensure(sys);
        let Some((t0, pi)) = self.playing else { return };
        if sys.now.saturating_sub(t0) <= self.total + 80 {
            return;
        }
        let next = pi + 1;
        if next < sys.settings.playlist.len() {
            self.start_at(sys, next);
        } else {
            // loop
            self.start_at(sys, 0);
        }
    }
    fn on_exit(&mut self, sys: &mut Sys) {
        sys.silence();
    }
    fn keep_awake(&self) -> bool {
        self.playing.is_some()
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, if self.show_pl { "PLAYLIST" } else { "ADD SONGS" });
        let n_pl = sys.settings.playlist.len();
        if self.show_pl {
            if n_pl == 0 {
                gfx::text_c(a, 64, 28, "empty", F::L7x13, true);
                gfx::text_c(a, 64, 44, "A:library B:add", F::S5x8, true);
            } else {
                let i = self.pl_i.min(n_pl - 1);
                let ti = sys.settings.playlist[i] as usize % TUNES.len();
                gfx::text(a, 4, 18, &format!("{}/{}", i + 1, n_pl), F::S5x8, true);
                gfx::text(a, 4, 28, &gfx::clip(TUNES[ti].name, 18), F::L7x13, true);
                if let Some((t0, pi)) = self.playing {
                    let el = (sys.now - t0).min(self.total);
                    gfx::bar(a, 4, 44, 120, 6, el as i32, self.total.max(1) as i32);
                    gfx::text(a, 4, 52, &format!("playing #{}", pi + 1), F::S4x6, true);
                }
            }
            gfx::hint(a, "#play B:rm C:clr A:lib");
        } else {
            let t = &TUNES[self.lib];
            gfx::text(a, 4, 18, &format!("{}/{}", self.lib + 1, TUNES.len()), F::S5x8, true);
            gfx::text(a, 4, 28, &gfx::clip(t.name, 18), F::L7x13, true);
            gfx::text(a, 4, 44, &format!("{} bpm  pl {}", t.bpm, n_pl), F::M6x10, true);
            gfx::hint(a, "#/B:add A:playlist");
        }
        aux_panel(b, "QUEUE", &[]);
        if n_pl == 0 {
            gfx::text_c(b, 64, 30, "no tracks yet", F::M6x10, true);
        } else {
            let top = if self.pl_i < 4 { 0 } else { self.pl_i - 3 };
            for row in 0..5 {
                let i = top + row;
                if i >= n_pl {
                    break;
                }
                let ti = sys.settings.playlist[i] as usize % TUNES.len();
                let y = 14 + row as i32 * 10;
                let on = i == self.pl_i.min(n_pl - 1) && self.show_pl;
                if on {
                    b.fill_rect(0, y - 1, 128, 10, true);
                }
                let mark = if self.playing.map(|(_, p)| p) == Some(i) { ">" } else { " " };
                gfx::text(b, 2, y, &format!("{}{}", mark, gfx::clip(TUNES[ti].name, 18)), F::M6x10, !on);
            }
        }
    }
}
