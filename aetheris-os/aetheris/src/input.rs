//! Keys, key events, debouncing/long-press/auto-repeat, and phone-style multi-tap text entry.
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    K0, K1, K2, K3, K4, K5, K6, K7, K8, K9,
    A, B, C, D, Star, Hash,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Nav {
    Up,
    Down,
    Left,
    Right,
    Ok,
    Back,
}

impl Key {
    pub fn digit(self) -> Option<u8> {
        use Key::*;
        Some(match self {
            K0 => 0, K1 => 1, K2 => 2, K3 => 3, K4 => 4, K5 => 5, K6 => 6, K7 => 7, K8 => 8, K9 => 9,
            _ => return None,
        })
    }
    pub fn from_digit(d: u8) -> Key {
        use Key::*;
        [K0, K1, K2, K3, K4, K5, K6, K7, K8, K9][(d % 10) as usize]
    }
    pub fn ch(self) -> char {
        match self {
            Key::A => 'A',
            Key::B => 'B',
            Key::C => 'C',
            Key::D => 'D',
            Key::Star => '*',
            Key::Hash => '#',
            k => (b'0' + k.digit().unwrap()) as char,
        }
    }
    /// Phone-style navigation: 2/8/4/6 or A/B/C/D arrows, 5 or # = OK, * = back.
    pub fn nav(self) -> Option<Nav> {
        Some(match self {
            Key::K2 | Key::A => Nav::Up,
            Key::K8 | Key::B => Nav::Down,
            Key::K4 | Key::C => Nav::Left,
            Key::K6 | Key::D => Nav::Right,
            Key::K5 | Key::Hash => Nav::Ok,
            Key::Star => Nav::Back,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Press,
    Repeat,
    Long,
    Release,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct KeyEvent {
    pub key: Key,
    pub kind: Kind,
}

impl KeyEvent {
    /// First press or auto-repeat (what menus want).
    pub fn down(&self) -> bool {
        matches!(self.kind, Kind::Press | Kind::Repeat)
    }
    pub fn press(&self) -> bool {
        self.kind == Kind::Press
    }
    pub fn long(&self) -> bool {
        self.kind == Kind::Long
    }
    pub fn release(&self) -> bool {
        self.kind == Kind::Release
    }
    pub fn nav(&self) -> Option<Nav> {
        if self.down() { self.key.nav() } else { None }
    }
}

/// Default 4x4 membrane layout: rows are driven, columns are read.
pub const KEYMAP: [[Key; 4]; 4] = [
    [Key::K1, Key::K2, Key::K3, Key::A],
    [Key::K4, Key::K5, Key::K6, Key::B],
    [Key::K7, Key::K8, Key::K9, Key::C],
    [Key::Star, Key::K0, Key::Hash, Key::D],
];

pub const LONG_MS: u64 = 700;
pub const REPEAT_DELAY_MS: u64 = 450;
pub const REPEAT_MS: u64 = 110;

/// Turns raw matrix samples (bit `r*4+c`) into debounced key events.
pub struct KeyScanner {
    stable: u16,
    last_raw: u16,
    since: [u64; 16],
    long_sent: [bool; 16],
    next_repeat: [u64; 16],
}

impl Default for KeyScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyScanner {
    pub fn new() -> Self {
        Self { stable: 0, last_raw: 0, since: [0; 16], long_sent: [false; 16], next_repeat: [0; 16] }
    }
    pub fn pressed_mask(&self) -> u16 {
        self.stable
    }
    pub fn key_at(idx: usize) -> Key {
        KEYMAP[idx / 4][idx % 4]
    }
    /// Feed one sample. Two identical consecutive samples are required (debounce).
    pub fn update(&mut self, raw: u16, now: u64, out: &mut Vec<KeyEvent>) {
        if raw != self.last_raw {
            self.last_raw = raw;
            return;
        }
        let changed = raw ^ self.stable;
        for i in 0..16 {
            let bit = 1u16 << i;
            let key = Self::key_at(i);
            if changed & bit != 0 {
                if raw & bit != 0 {
                    self.since[i] = now;
                    self.long_sent[i] = false;
                    self.next_repeat[i] = now + REPEAT_DELAY_MS;
                    out.push(KeyEvent { key, kind: Kind::Press });
                } else {
                    out.push(KeyEvent { key, kind: Kind::Release });
                }
            } else if raw & bit != 0 {
                if !self.long_sent[i] && now - self.since[i] >= LONG_MS {
                    self.long_sent[i] = true;
                    out.push(KeyEvent { key, kind: Kind::Long });
                }
                if now >= self.next_repeat[i] {
                    self.next_repeat[i] = now + REPEAT_MS;
                    if key.nav().is_some() && !matches!(key, Key::Star | Key::Hash | Key::K5) {
                        out.push(KeyEvent { key, kind: Kind::Repeat });
                    }
                }
            }
        }
        self.stable = raw;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Lower,
    Upper,
    Digits,
}

impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Mode::Lower => "abc",
            Mode::Upper => "ABC",
            Mode::Digits => "123",
        }
    }
    pub fn next(self) -> Mode {
        match self {
            Mode::Lower => Mode::Upper,
            Mode::Upper => Mode::Digits,
            Mode::Digits => Mode::Lower,
        }
    }
}

pub fn tap_chars(k: Key) -> &'static str {
    match k {
        Key::K1 => ".,?!1@-_/:;()&$%+=#'\"",
        Key::K2 => "abc2",
        Key::K3 => "def3",
        Key::K4 => "ghi4",
        Key::K5 => "jkl5",
        Key::K6 => "mno6",
        Key::K7 => "pqrs7",
        Key::K8 => "tuv8",
        Key::K9 => "wxyz9",
        Key::K0 => " 0",
        _ => "",
    }
}

pub const TAP_TIMEOUT_MS: u64 = 1000;

/// Multi-tap text field (like old button phones).
pub struct TextEntry {
    pub text: String,
    pub mode: Mode,
    pub max: usize,
    pending: Option<(Key, usize, u64)>,
}

impl TextEntry {
    pub fn new(max: usize) -> Self {
        Self { text: String::new(), mode: Mode::Lower, max, pending: None }
    }
    pub fn with_text(max: usize, s: &str) -> Self {
        let mut t = Self::new(max);
        t.text = s.chars().take(max).collect();
        t
    }
    /// Returns true if the key was consumed as text input.
    pub fn key(&mut self, k: Key, now: u64) -> bool {
        if k.digit().is_none() {
            return false;
        }
        if self.mode == Mode::Digits {
            self.pending = None;
            if self.text.chars().count() < self.max {
                self.text.push(k.ch());
            }
            return true;
        }
        let set = tap_chars(k);
        let n = set.chars().count();
        if let Some((pk, idx, t)) = self.pending
            && pk == k
            && now.saturating_sub(t) < TAP_TIMEOUT_MS
        {
            let nidx = (idx + 1) % n;
            self.text.pop();
            self.text.push(self.case(set.chars().nth(nidx).unwrap()));
            self.pending = Some((k, nidx, now));
            return true;
        }
        if self.text.chars().count() >= self.max {
            self.pending = None;
            return true;
        }
        self.text.push(self.case(set.chars().next().unwrap()));
        self.pending = Some((k, 0, now));
        true
    }
    fn case(&self, c: char) -> char {
        if self.mode == Mode::Upper { c.to_ascii_uppercase() } else { c }
    }
    pub fn backspace(&mut self) {
        self.pending = None;
        self.text.pop();
    }
    pub fn clear(&mut self) {
        self.pending = None;
        self.text.clear();
    }
    pub fn cycle_mode(&mut self) {
        self.pending = None;
        self.mode = self.mode.next();
    }
    pub fn commit(&mut self) {
        self.pending = None;
    }
    pub fn tick(&mut self, now: u64) {
        if let Some((_, _, t)) = self.pending
            && now.saturating_sub(t) >= TAP_TIMEOUT_MS
        {
            self.pending = None;
        }
    }
    /// The character that is still being cycled (shown highlighted).
    pub fn pending(&self) -> bool {
        self.pending.is_some()
    }
    pub fn set_text(&mut self, s: &str) {
        self.pending = None;
        self.text = s.chars().take(self.max).collect();
    }
}
