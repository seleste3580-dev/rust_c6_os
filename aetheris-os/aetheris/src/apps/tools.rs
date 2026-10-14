//! Everyday tools: calculator, clock, stopwatch, timer, focus (pomodoro), calendar, unit converter, notes, dice, system info, key test, torch.
use super::{App, Flow, aux_panel, fmt_ms};
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::input::{Key, KeyEvent, Mode, TextEntry};
use crate::sound::Snd;
use crate::sys::Sys;
use crate::trig::*;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

const MICRO: i64 = 1_000_000;

// ───────────────────────────── Calculator ─────────────────────────────
pub struct Calc {
    acc: i64,
    cur: String,
    op: Option<char>,
    fresh: bool,
    err: bool,
    hist: Vec<String>,
    show: i64,
}

fn parse_micro(s: &str) -> i64 {
    let (ip, fp) = match s.split_once('.') {
        Some((a, b)) => (a, b),
        None => (s, ""),
    };
    let mut v: i64 = ip.parse().unwrap_or(0);
    v = v.saturating_mul(MICRO);
    let mut f = 0i64;
    let mut scale = MICRO / 10;
    for c in fp.chars().take(6) {
        f += (c as i64 - '0' as i64) * scale;
        scale /= 10;
    }
    v + f
}

pub fn fmt_micro(v: i64) -> String {
    let neg = v < 0;
    let a = v.unsigned_abs();
    let ip = a / MICRO as u64;
    let fp = a % MICRO as u64;
    let mut s = String::new();
    if neg {
        s.push('-');
    }
    s.push_str(&format!("{}", ip));
    if fp != 0 {
        let f = format!("{:06}", fp);
        s.push('.');
        s.push_str(f.trim_end_matches('0'));
    }
    s
}

impl Calc {
    pub fn new() -> Self {
        Self { acc: 0, cur: String::new(), op: None, fresh: true, err: false, hist: Vec::new(), show: 0 }
    }
    fn apply(&mut self) {
        let b = if self.cur.is_empty() { self.acc } else { parse_micro(&self.cur) };
        if let Some(op) = self.op {
            let a = self.acc as i128;
            let bb = b as i128;
            let r: Option<i128> = match op {
                '+' => Some(a + bb),
                '-' => Some(a - bb),
                'x' => Some(a * bb / MICRO as i128),
                '/' => {
                    if b == 0 { None } else { Some(a * MICRO as i128 / bb) }
                }
                _ => Some(bb),
            };
            match r.and_then(|v| i64::try_from(v).ok()) {
                Some(v) => {
                    self.hist.push(format!("{} {} {} = {}", fmt_micro(self.acc), op, fmt_micro(b), fmt_micro(v)));
                    if self.hist.len() > 5 {
                        self.hist.remove(0);
                    }
                    self.acc = v;
                }
                None => self.err = true,
            }
        } else {
            self.acc = b;
        }
        self.cur.clear();
        self.show = self.acc;
    }
}

impl App for Calc {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.long() && ev.key == Key::Hash {
            *self = Calc::new();
            sys.snd(Snd::Back);
            return Flow::Stay;
        }
        if !ev.press() {
            return Flow::Stay;
        }
        if self.err {
            *self = Calc::new();
        }
        match ev.key {
            k if k.digit().is_some() => {
                if self.fresh && self.op.is_none() {
                    self.cur.clear();
                }
                if self.cur.len() < 11 {
                    if !(self.cur == "0" && k == Key::K0) {
                        if self.cur == "0" {
                            self.cur.clear();
                        }
                        self.cur.push(k.ch());
                    }
                }
                self.fresh = false;
                sys.snd(Snd::Click);
            }
            Key::Star => {
                if !self.cur.contains('.') {
                    if self.cur.is_empty() {
                        self.cur.push('0');
                    }
                    self.cur.push('.');
                }
                sys.snd(Snd::Click);
            }
            Key::A | Key::B | Key::C | Key::D => {
                let op = match ev.key {
                    Key::A => '+',
                    Key::B => '-',
                    Key::C => 'x',
                    _ => '/',
                };
                if !self.cur.is_empty() || self.op.is_none() {
                    self.apply();
                }
                self.op = Some(op);
                self.fresh = false;
                sys.snd(Snd::Select);
            }
            Key::Hash => {
                self.apply();
                self.op = None;
                self.fresh = true;
                sys.snd(if self.err { Snd::Error } else { Snd::Accept });
            }
            _ => {}
        }
        Flow::Stay
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, "CALC");
        let big = if self.err {
            String::from("Error")
        } else if !self.cur.is_empty() {
            self.cur.clone()
        } else {
            fmt_micro(self.show)
        };
        let top = match self.op {
            Some(o) if !self.err => format!("{} {}", fmt_micro(self.acc), o),
            _ => String::new(),
        };
        gfx::text_r(a, 124, 19, &gfx::clip(&top, 20), F::M6x10, true);
        let f = if big.len() > 12 { F::M6x10 } else { F::H10x20 };
        gfx::text_r(a, 124, 32, &big, f, true);
        gfx::hint(a, "A+ B- Cx D/ *. #= hold#:AC");
        aux_panel(b, "HISTORY", &[]);
        for (i, h) in self.hist.iter().rev().enumerate() {
            gfx::text_r(b, 125, 16 + i as i32 * 9, &gfx::clip(h, 21), F::S5x8, true);
        }
    }
}

// ───────────────────────────── Clock ─────────────────────────────
pub struct ClockApp {
    h12: bool,
}
impl ClockApp {
    pub fn new() -> Self {
        Self { h12: false }
    }
}
impl App for ClockApp {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.press() && matches!(ev.key, Key::K5 | Key::Hash) {
            self.h12 = !self.h12;
            sys.snd(Snd::Select);
        }
        if ev.press() && ev.key == Key::D {
            return Flow::Exit;
        }
        Flow::Stay
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        let t = sys.local_time();
        gfx::title(a, "CLOCK");
        let (h, suffix) = if self.h12 { (if t.h % 12 == 0 { 12 } else { t.h % 12 }, if t.h >= 12 { "PM" } else { "AM" }) } else { (t.h, "") };
        let s = format!("{:02}:{:02}:{:02}", h, t.mi, t.s);
        gfx::text_c(a, 60, 21, &s, F::H10x20, true);
        gfx::text(a, 112, 30, suffix, F::S5x8, true);
        let d = format!("{} {} {} {}", crate::clock::DAYS[t.wd as usize], t.d, crate::clock::MONTHS[(t.mo - 1) as usize], t.y);
        gfx::text_c(a, 64, 44, &d, F::M6x10, true);
        gfx::hint(a, if sys.wifi.time_synced { "#:12/24h  synced by Wi-Fi" } else { "#:12/24h  set in Settings" });
        // analog face
        b.circle(64, 32, 30, true);
        for i in 0..12 {
            let k = i * 5;
            let (c, s) = (COS60[k as usize] as i32, SIN60[k as usize] as i32);
            b.line(64 + s * 26 / 100, 32 - c * 26 / 100, 64 + s * 29 / 100, 32 - c * 29 / 100, true);
        }
        let hand = |b: &mut Fb, idx: i32, len: i32| {
            let i = idx.rem_euclid(60) as usize;
            b.line(64, 32, 64 + SIN60[i] as i32 * len / 100, 32 - COS60[i] as i32 * len / 100, true);
        };
        hand(b, (t.h as i32 % 12) * 5 + t.mi as i32 / 12, 16);
        hand(b, t.mi as i32, 24);
        hand(b, t.s as i32, 27);
        b.disc(64, 32, 2, true);
    }
}

// ───────────────────────────── Stopwatch ─────────────────────────────
pub struct Stopwatch {
    running: bool,
    start: u64,
    acc: u64,
    laps: Vec<u64>,
}
impl Stopwatch {
    pub fn new() -> Self {
        Self { running: false, start: 0, acc: 0, laps: Vec::new() }
    }
    fn elapsed(&self, now: u64) -> u64 {
        self.acc + if self.running { now - self.start } else { 0 }
    }
}
impl App for Stopwatch {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.press() {
            return Flow::Stay;
        }
        match ev.key {
            Key::K5 | Key::Hash => {
                if self.running {
                    self.acc = self.elapsed(sys.now);
                    self.running = false;
                } else {
                    self.start = sys.now;
                    self.running = true;
                }
                sys.snd(Snd::Select);
            }
            Key::A | Key::K2 => {
                if self.running && self.laps.len() < 50 {
                    self.laps.push(self.elapsed(sys.now));
                    sys.snd(Snd::Ping);
                }
            }
            Key::B | Key::K8 | Key::K0 => {
                if !self.running {
                    self.acc = 0;
                    self.laps.clear();
                    sys.snd(Snd::Back);
                }
            }
            Key::D => return Flow::Exit,
            _ => {}
        }
        Flow::Stay
    }
    fn keep_awake(&self) -> bool {
        self.running
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "STOPWATCH");
        gfx::text_c(a, 64, 24, &fmt_ms(self.elapsed(sys.now)), F::H10x20, true);
        gfx::hint(a, "#:go/stop A:lap B:reset");
        aux_panel(b, "LAPS", &[]);
        for (i, l) in self.laps.iter().rev().take(5).enumerate() {
            let n = self.laps.len() - i;
            gfx::text(b, 4, 16 + i as i32 * 9, &format!("#{:<2} {}", n, fmt_ms(*l)), F::M6x10, true);
        }
    }
}

// ───────────────────────────── Timer ─────────────────────────────
#[derive(PartialEq, Eq, Clone, Copy)]
enum TState {
    Set,
    Run,
    Pause,
    Done,
}
pub struct Timer {
    digits: String,
    st: TState,
    total: u64,
    left: u64,
    last: u64,
    next_alarm: u64,
}
impl Timer {
    pub fn new() -> Self {
        Self { digits: String::new(), st: TState::Set, total: 0, left: 0, last: 0, next_alarm: 0 }
    }
    fn entered_ms(&self) -> u64 {
        let v: u64 = self.digits.parse().unwrap_or(0);
        let (m, s) = (v / 100, v % 100);
        (m.min(99) * 60 + s.min(59)) * 1000
    }
}
impl App for Timer {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.press() {
            return Flow::Stay;
        }
        if self.st == TState::Done {
            self.st = TState::Set;
            self.digits.clear();
            sys.silence();
            return Flow::Stay;
        }
        match ev.key {
            k if k.digit().is_some() && self.st == TState::Set => {
                if self.digits.len() < 4 {
                    self.digits.push(k.ch());
                    sys.snd(Snd::Click);
                }
            }
            Key::A | Key::B | Key::C if self.st == TState::Set => {
                let m = match ev.key {
                    Key::A => 1,
                    Key::B => 5,
                    _ => 10,
                };
                self.digits = format!("{:02}00", m);
                sys.snd(Snd::Select);
            }
            Key::Star => {
                if self.st == TState::Set {
                    self.digits.pop();
                } else {
                    self.st = TState::Set;
                }
                sys.snd(Snd::Back);
            }
            Key::Hash | Key::K5 => match self.st {
                TState::Set => {
                    self.total = self.entered_ms();
                    if self.total > 0 {
                        self.left = self.total;
                        self.last = sys.now;
                        self.st = TState::Run;
                        sys.snd(Snd::Accept);
                    } else {
                        sys.snd(Snd::Error);
                    }
                }
                TState::Run => self.st = TState::Pause,
                TState::Pause => {
                    self.last = sys.now;
                    self.st = TState::Run;
                }
                TState::Done => {}
            },
            Key::D => return Flow::Exit,
            _ => {}
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.st == TState::Run {
            let d = sys.now - self.last;
            self.last = sys.now;
            if d >= self.left {
                self.left = 0;
                self.st = TState::Done;
                self.next_alarm = sys.now;
            } else {
                self.left -= d;
            }
        }
        if self.st == TState::Done && sys.now >= self.next_alarm {
            sys.snd(Snd::Alarm);
            sys.led_pulse(1200);
            self.next_alarm = sys.now + 2600;
        }
    }
    fn keep_awake(&self) -> bool {
        matches!(self.st, TState::Run | TState::Done)
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, "TIMER");
        let ms = match self.st {
            TState::Set => self.entered_ms(),
            _ => self.left,
        };
        let s = format!("{:02}:{:02}", ms / 60000, ms / 1000 % 60);
        gfx::text_c(a, 64, 24, &s, F::H10x20, true);
        let hint = match self.st {
            TState::Set => "mmss A:1m B:5m C:10m #go",
            TState::Run => "#:pause  *:stop",
            TState::Pause => "#:resume  *:stop",
            TState::Done => "TIME UP! any key",
        };
        gfx::hint(a, hint);
        // hourglass progress on aux
        aux_panel(b, "TIMER", &[]);
        let frac = if self.total == 0 { 0 } else { (self.left * 100 / self.total) as i32 };
        gfx::bar(b, 8, 24, 112, 14, frac, 100);
        gfx::text_c(b, 64, 44, &format!("{}% left", frac), F::M6x10, true);
        if self.st == TState::Done {
            b.invert_rect(0, 0, 128, 64);
        }
    }
}

// ───────────────────────────── Focus (Pomodoro) ─────────────────────────────
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Work,
    Short,
    Long,
}
pub struct Focus {
    work_min: u8,
    brk_min: u8,
    phase: Phase,
    running: bool,
    left: u64,
    last: u64,
    /// Completed work sessions (a long break follows every 4th).
    done: u32,
    focused_ms: u64,
}
impl Focus {
    pub fn new() -> Self {
        let mut f = Self { work_min: 25, brk_min: 5, phase: Phase::Work, running: false, left: 0, last: 0, done: 0, focused_ms: 0 };
        f.left = f.phase_ms();
        f
    }
    fn phase_ms(&self) -> u64 {
        let m = match self.phase {
            Phase::Work => self.work_min,
            Phase::Short => self.brk_min,
            Phase::Long => self.brk_min.saturating_mul(3).min(30),
        };
        m as u64 * 60_000
    }
    fn label(&self) -> &'static str {
        match self.phase {
            Phase::Work => "FOCUS",
            Phase::Short => "SHORT BREAK",
            Phase::Long => "LONG BREAK",
        }
    }
    /// Move to the next phase; `completed` = the current phase ran to the end.
    fn advance(&mut self, completed: bool) {
        self.phase = match self.phase {
            Phase::Work => {
                if completed {
                    self.done += 1;
                }
                if self.done > 0 && self.done % 4 == 0 { Phase::Long } else { Phase::Short }
            }
            _ => Phase::Work,
        };
        self.left = self.phase_ms();
    }
}
impl App for Focus {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.down() {
            return Flow::Stay;
        }
        let idle = !self.running && self.left == self.phase_ms();
        match ev.key {
            Key::Hash | Key::K5 if ev.press() => {
                self.running = !self.running;
                self.last = sys.now;
                sys.snd(if self.running { Snd::Accept } else { Snd::Select });
            }
            // lengths can only be edited before a phase starts
            Key::K4 | Key::K6 if idle => {
                self.work_min = if ev.key == Key::K6 { (self.work_min + 5).min(90) } else { self.work_min.saturating_sub(5).max(5) };
                self.left = self.phase_ms();
                sys.snd(Snd::Click);
            }
            Key::K2 | Key::K8 if idle => {
                self.brk_min = if ev.key == Key::K2 { (self.brk_min + 1).min(30) } else { self.brk_min.saturating_sub(1).max(1) };
                self.left = self.phase_ms();
                sys.snd(Snd::Click);
            }
            Key::B if ev.press() => {
                self.advance(false);
                self.running = false;
                sys.snd(Snd::Select);
            }
            Key::A if ev.press() => {
                self.running = false;
                self.left = self.phase_ms();
                sys.snd(Snd::Back);
            }
            Key::C if ev.press() => {
                *self = Self { work_min: self.work_min, brk_min: self.brk_min, ..Self::new() };
                self.left = self.phase_ms();
                sys.snd(Snd::Back);
            }
            Key::D if ev.press() => return Flow::Exit,
            _ => {}
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if !self.running {
            return;
        }
        let d = sys.now - self.last;
        self.last = sys.now;
        if self.phase == Phase::Work {
            self.focused_ms += d.min(self.left);
        }
        if d >= self.left {
            let was_work = self.phase == Phase::Work;
            self.advance(true);
            // chime, flash the LED and roll straight into the next phase
            sys.snd(if was_work { Snd::Win } else { Snd::Notify });
            sys.led_pulse(1500);
        } else {
            self.left -= d;
        }
    }
    fn keep_awake(&self) -> bool {
        self.running
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, self.label());
        let s = format!("{:02}:{:02}", self.left / 60000, self.left / 1000 % 60);
        gfx::text_c(a, 64, 20, &s, F::H10x20, true);
        if !self.running && self.left != self.phase_ms() {
            gfx::text_c(a, 64, 42, "paused", F::S5x8, true);
        } else {
            gfx::text_c(a, 64, 42, &format!("work {}m  break {}m", self.work_min, self.brk_min), F::S5x8, true);
        }
        let idle = !self.running && self.left == self.phase_ms();
        gfx::hint(a, if idle { "#go 4/6 work 2/8 break" } else if self.running { "#pause B:skip A:restart" } else { "#resume B:skip C:reset" });

        aux_panel(b, "POMODORO", &[]);
        let total = self.phase_ms().max(1);
        gfx::bar(b, 6, 16, 116, 10, ((total - self.left) * 100 / total) as i32, 100);
        // four "tomatoes": filled = sessions done in the current set
        let in_set = if self.done > 0 && self.done % 4 == 0 && self.phase == Phase::Long { 4 } else { self.done % 4 };
        for k in 0..4 {
            let x = 22 + k * 28;
            if (k as u32) < in_set {
                b.disc(x, 37, 5, true);
            } else {
                b.circle(x, 37, 5, true);
            }
        }
        let mins = self.focused_ms / 60000;
        gfx::text_c(b, 64, 50, &format!("{} done  {}h{:02}m", self.done, mins / 60, mins % 60), F::M6x10, true);
    }
}

// ───────────────────────────── Calendar ─────────────────────────────
pub struct Calendar {
    /// Selected day as days since 1970-01-01 (local time); `None` until the first tick.
    sel: Option<i64>,
}
impl Calendar {
    pub fn new() -> Self {
        Self { sel: None }
    }
    fn today(sys: &Sys) -> i64 {
        sys.local_epoch().div_euclid(86_400)
    }
    fn date(day: i64) -> crate::clock::DateTime {
        crate::clock::from_epoch(day * 86_400)
    }
    fn day_of(y: i32, mo: u8, d: u8) -> i64 {
        crate::clock::to_epoch(y, mo, d, 0, 0, 0).div_euclid(86_400)
    }
    /// Same day-of-month `dm` months away, clamped to the length of the target month.
    fn add_months(day: i64, dm: i32) -> i64 {
        let t = Self::date(day);
        let m0 = t.y * 12 + t.mo as i32 - 1 + dm;
        let (y, mo) = (m0.div_euclid(12), (m0.rem_euclid(12) + 1) as u8);
        Self::day_of(y, mo, t.d.min(crate::clock::days_in_month(y, mo)))
    }
    /// ISO-8601 week number (weeks start on Monday, week 1 holds the first Thursday).
    fn iso_week(day: i64) -> u32 {
        let wd = (day + 3).rem_euclid(7); // 0 = Monday
        let thu = day - wd + 3;
        let y = Self::date(thu).y;
        ((thu - Self::day_of(y, 1, 1)) / 7 + 1) as u32
    }
}
impl App for Calendar {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.down() {
            return Flow::Stay;
        }
        let day = self.sel.unwrap_or_else(|| Self::today(sys));
        let next = match ev.key {
            Key::K4 => day - 1,
            Key::K6 => day + 1,
            Key::K2 => day - 7,
            Key::K8 => day + 7,
            Key::A => Self::add_months(day, -1),
            Key::B => Self::add_months(day, 1),
            Key::K1 => Self::add_months(day, -12),
            Key::K3 => Self::add_months(day, 12),
            Key::C | Key::K5 | Key::Hash => Self::today(sys),
            Key::D if ev.press() => return Flow::Exit,
            _ => return Flow::Stay,
        };
        self.sel = Some(next);
        sys.snd(Snd::Click);
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.sel.is_none() {
            self.sel = Some(Self::today(sys));
        }
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        use crate::clock::{DAYS, MONTHS, days_in_month};
        let today = Self::today(sys);
        let day = self.sel.unwrap_or(today);
        let t = Self::date(day);
        gfx::title(a, "CALENDAR");
        gfx::text_c(a, 64, 18, &format!("{} {} {}", DAYS[t.wd as usize], t.d, MONTHS[(t.mo - 1) as usize]), F::B9x15, true);
        let doy = day - Self::day_of(t.y, 1, 1) + 1;
        gfx::text_c(a, 64, 33, &format!("{}  day {}  wk {}", t.y, doy, Self::iso_week(day)), F::M6x10, true);
        let rel = match day - today {
            0 => String::from("today"),
            1 => String::from("tomorrow"),
            -1 => String::from("yesterday"),
            n if n > 0 => format!("in {} days", n),
            n => format!("{} days ago", -n),
        };
        gfx::text_c(a, 64, 44, &rel, F::S5x8, true);
        gfx::hint(a, "46 day 28 wk AB mon C now");

        // month grid on the second screen, weeks start on Monday
        b.fill_rect(0, 0, 128, 9, true);
        gfx::text_c(b, 64, 1, &format!("{} {}", MONTHS[(t.mo - 1) as usize], t.y), F::S5x8, false);
        for (c, l) in ["M", "T", "W", "T", "F", "S", "S"].iter().enumerate() {
            gfx::text_c(b, 10 + c as i32 * 18, 10, l, F::S4x6, true);
        }
        let first = Self::day_of(t.y, t.mo, 1);
        let lead = (first + 3).rem_euclid(7) as i32; // Monday = 0
        let td = Self::date(today);
        for d in 1..=days_in_month(t.y, t.mo) as i32 {
            let cell = lead + d - 1;
            let (x, y) = (1 + (cell % 7) * 18, 17 + (cell / 7) * 8);
            let on = d == t.d as i32;
            if on {
                b.fill_rect(x, y - 1, 17, 8, true);
            } else if td.y == t.y && td.mo == t.mo && td.d as i32 == d {
                b.rect(x, y - 1, 17, 8, true);
            }
            gfx::text_c(b, x + 9, y, &format!("{}", d), F::S4x6, !on);
        }
    }
}

// ───────────────────────────── Unit converter ─────────────────────────────
/// (unit, factor to the category's base unit, offset added after scaling) — base = v * factor + offset.
type Unit = (&'static str, f64, f64);
const CONV: &[(&str, &[Unit])] = &[
    ("LENGTH", &[("m", 1.0, 0.0), ("cm", 0.01, 0.0), ("mm", 0.001, 0.0), ("km", 1000.0, 0.0), ("in", 0.0254, 0.0), ("ft", 0.3048, 0.0), ("yd", 0.9144, 0.0), ("mi", 1609.344, 0.0)]),
    ("MASS", &[("kg", 1.0, 0.0), ("g", 0.001, 0.0), ("t", 1000.0, 0.0), ("lb", 0.453_592_37, 0.0), ("oz", 0.028_349_523_125, 0.0)]),
    ("TEMP", &[("C", 1.0, 273.15), ("F", 5.0 / 9.0, 273.15 - 32.0 * 5.0 / 9.0), ("K", 1.0, 0.0)]),
    ("VOLUME", &[("L", 1.0, 0.0), ("ml", 0.001, 0.0), ("m3", 1000.0, 0.0), ("gal", 3.785_411_784, 0.0), ("cup", 0.236_588_236_5, 0.0), ("floz", 0.029_573_529_562_5, 0.0)]),
    ("SPEED", &[("km/h", 1.0 / 3.6, 0.0), ("m/s", 1.0, 0.0), ("mph", 0.447_04, 0.0), ("knot", 1852.0 / 3600.0, 0.0)]),
    ("AREA", &[("m2", 1.0, 0.0), ("cm2", 1e-4, 0.0), ("km2", 1e6, 0.0), ("ha", 1e4, 0.0), ("acre", 4046.856_422_4, 0.0), ("ft2", 0.092_903_04, 0.0)]),
    ("DATA", &[("B", 1.0, 0.0), ("KB", 1024.0, 0.0), ("MB", 1_048_576.0, 0.0), ("GB", 1_073_741_824.0, 0.0), ("bit", 0.125, 0.0)]),
    ("ENERGY", &[("J", 1.0, 0.0), ("kJ", 1000.0, 0.0), ("cal", 4.184, 0.0), ("kcal", 4184.0, 0.0), ("Wh", 3600.0, 0.0), ("kWh", 3.6e6, 0.0)]),
];

fn fmt_num(v: f64) -> String {
    if !v.is_finite() {
        return String::from("---");
    }
    let a = if v < 0.0 { -v } else { v };
    if a != 0.0 && !(1e-4..1e9).contains(&a) {
        return format!("{:.3e}", v);
    }
    let s = format!("{:.6}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    // keep about 9 significant characters
    let s = if s.len() > 10 && s.contains('.') { s[..10].trim_end_matches('.') } else { s };
    if s == "-0" { String::from("0") } else { String::from(s) }
}

pub struct Converter {
    cat: usize,
    from: usize,
    to: usize,
    input: String,
    neg: bool,
}
impl Converter {
    pub fn new() -> Self {
        Self { cat: 0, from: 0, to: 5, input: String::from("1"), neg: false }
    }
    fn value(&self) -> f64 {
        let v: f64 = self.input.parse().unwrap_or(0.0);
        if self.neg { -v } else { v }
    }
    fn convert(&self, to: usize) -> f64 {
        let units = CONV[self.cat].1;
        let (_, ff, fo) = units[self.from];
        let (_, tf, to_off) = units[to];
        (self.value() * ff + fo - to_off) / tf
    }
}
impl App for Converter {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.long() {
            match ev.key {
                Key::Hash => self.input.clear(),
                Key::Star => {
                    // the last short press already typed a '.', undo it before flipping the sign
                    if self.input.ends_with('.') {
                        self.input.pop();
                    }
                    self.neg = !self.neg;
                }
                _ => return Flow::Stay,
            }
            sys.snd(Snd::Back);
            return Flow::Stay;
        }
        if !ev.press() {
            return Flow::Stay;
        }
        let n = CONV[self.cat].1.len();
        match ev.key {
            k if k.digit().is_some() => {
                if self.input.len() < 10 {
                    if self.input == "0" {
                        self.input.clear();
                    }
                    self.input.push(k.ch());
                }
            }
            Key::Star => {
                if !self.input.contains('.') && self.input.len() < 9 {
                    if self.input.is_empty() {
                        self.input.push('0');
                    }
                    self.input.push('.');
                }
            }
            Key::Hash => {
                self.input.pop();
            }
            Key::A => {
                self.cat = (self.cat + 1) % CONV.len();
                self.from = 0;
                self.to = 1.min(CONV[self.cat].1.len() - 1);
            }
            Key::B => self.from = (self.from + 1) % n,
            Key::C => self.to = (self.to + 1) % n,
            Key::D => return Flow::Exit,
            _ => return Flow::Stay,
        }
        sys.snd(Snd::Click);
        Flow::Stay
    }
    fn owns_star(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        let (cat, units) = CONV[self.cat];
        gfx::title(a, cat);
        let shown = if self.input.is_empty() { "0" } else { &self.input };
        let inp = format!("{}{} {}", if self.neg { "-" } else { "" }, shown, units[self.from].0);
        gfx::text_r(a, 124, 18, &gfx::clip(&inp, 17), F::L7x13, true);
        a.hline(4, 33, 120, true);
        let out = format!("= {} {}", fmt_num(self.convert(self.to)), units[self.to].0);
        // big font when it fits, otherwise fall back to a narrower one
        let f = if gfx::text_w(&out, F::B9x15) <= 122 { F::B9x15 } else { F::L7x13 };
        gfx::text_r(a, 124, 37, &gfx::clip(&out, 17), f, true);
        gfx::hint(a, "A type B from C to #del");

        // every other unit of the category on the second screen
        b.fill_rect(0, 0, 128, 9, true);
        gfx::text_c(b, 64, 1, &format!("{} {} =", fmt_num(self.value()), units[self.from].0), F::S5x8, false);
        let mut row = 0;
        for (i, u) in units.iter().enumerate() {
            if i == self.from || row >= 5 {
                continue;
            }
            let y = 11 + row * 10;
            gfx::text(b, 3, y, &gfx::clip(&fmt_num(self.convert(i)), 14), F::M6x10, true);
            gfx::text_r(b, 125, y, u.0, F::M6x10, true);
            if i == self.to {
                b.invert_rect(0, y - 1, 128, 10);
            }
            row += 1;
        }
        if units.len() <= 5 {
            gfx::text_c(b, 64, 56, "*: .  hold*: +/-  D:exit", F::S4x6, true);
        }
    }
}

// ───────────────────────────── Notes ─────────────────────────────
pub struct Notes {
    e: TextEntry,
    saved: bool,
}
impl Notes {
    pub fn new() -> Self {
        Self { e: TextEntry::new(600), saved: true }
    }
}
fn wrap(s: &str, w: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if cur.chars().count() >= w {
            out.push(core::mem::take(&mut cur));
        }
        cur.push(c);
    }
    out.push(cur);
    out
}
impl App for Notes {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if self.e.text.is_empty() && self.saved && !sys.settings.note.is_empty() && self.e.text != sys.settings.note {
            self.e.set_text(&sys.settings.note.clone());
        }
        if ev.long() && ev.key == Key::Star {
            return Flow::Exit; // hold * handled globally too; also acts as save+exit
        }
        if !ev.press() {
            return Flow::Stay;
        }
        match ev.key {
            Key::Hash => {
                self.e.commit();
                sys.settings.note = self.e.text.clone();
                sys.save();
                sys.snd(Snd::Accept);
                self.saved = true;
            }
            Key::Star => {
                self.e.backspace();
                self.saved = false;
                sys.snd(Snd::Typing);
            }
            Key::A => {
                self.e.cycle_mode();
                sys.snd(Snd::Click);
            }
            Key::B => {
                self.e.clear();
                self.saved = false;
                sys.snd(Snd::Back);
            }
            Key::D => return Flow::Exit,
            Key::C => {}
            k => {
                if self.e.key(k, sys.now) {
                    self.saved = false;
                    sys.snd(Snd::Typing);
                }
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.e.text.is_empty() && self.saved && !sys.settings.note.is_empty() {
            self.e.set_text(&sys.settings.note.clone());
        }
        self.e.tick(sys.now);
    }
    fn on_exit(&mut self, sys: &mut Sys) {
        if !self.saved {
            sys.settings.note = self.e.text.clone();
            sys.save();
        }
    }
    fn owns_star(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "NOTES");
        let lines = wrap(&self.e.text, 21);
        let n = lines.len();
        for (i, l) in lines.iter().skip(n.saturating_sub(3)).enumerate() {
            gfx::text(a, 2, 18 + i as i32 * 12, l, F::M6x10, true);
        }
        if (sys.now / 450) % 2 == 0 {
            let last = lines.last().map(|l| l.chars().count()).unwrap_or(0) as i32;
            let row = (n.min(3) - 1) as i32;
            a.vline(2 + last * 6, 18 + row * 12, 10, true);
        }
        gfx::text_r(a, 124, 3, self.e.mode.label(), F::M6x10, true);
        gfx::hint(a, if self.saved { "saved #save A:abc B:clr" } else { "EDITED #save A:abc B:clr" });
        let full = wrap(&self.e.text, 25);
        let m = full.len();
        for (i, l) in full.iter().skip(m.saturating_sub(7)).enumerate() {
            gfx::text(b, 1, 1 + i as i32 * 9, l, F::S5x8, true);
        }
        gfx::text_r(b, 127, 56, &format!("{}/600", self.e.text.chars().count()), F::S5x8, true);
        let _ = Mode::Lower;
    }
}

// ───────────────────────────── Dice ─────────────────────────────
pub struct Dice {
    mode: u8,
    n: u8,
    vals: [u8; 3],
    anim: u8,
    coin: bool,
    rnd: u32,
}
impl Dice {
    pub fn new() -> Self {
        Self { mode: 0, n: 1, vals: [1; 3], anim: 0, coin: false, rnd: 0 }
    }
    const MODES: [&'static str; 4] = ["D6", "D20", "Coin", "1-100"];
}
fn die(b: &mut Fb, x: i32, y: i32, s: i32, v: u8) {
    gfx::frame(b, x, y, s, s);
    let q = s / 4;
    let pip = |b: &mut Fb, cx: i32, cy: i32| b.disc(x + cx * q, y + cy * q, (s / 12).max(1), true);
    match v {
        1 => pip(b, 2, 2),
        2 => { pip(b, 1, 1); pip(b, 3, 3) }
        3 => { pip(b, 1, 1); pip(b, 2, 2); pip(b, 3, 3) }
        4 => { pip(b, 1, 1); pip(b, 3, 1); pip(b, 1, 3); pip(b, 3, 3) }
        5 => { pip(b, 1, 1); pip(b, 3, 1); pip(b, 2, 2); pip(b, 1, 3); pip(b, 3, 3) }
        _ => { pip(b, 1, 1); pip(b, 3, 1); pip(b, 1, 2); pip(b, 3, 2); pip(b, 1, 3); pip(b, 3, 3) }
    }
}
impl App for Dice {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.press() {
            return Flow::Stay;
        }
        match ev.key {
            Key::K5 | Key::Hash => {
                self.anim = 14;
                sys.snd(Snd::Click);
            }
            Key::A | Key::K4 | Key::K6 | Key::C | Key::D if ev.key != Key::D => {
                self.mode = (self.mode + 1) % 4;
                sys.snd(Snd::Select);
            }
            Key::B | Key::K2 | Key::K8 => {
                self.n = self.n % 3 + 1;
                sys.snd(Snd::Select);
            }
            Key::D => return Flow::Exit,
            _ => {}
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if self.anim > 0 {
            self.anim -= 1;
            let sides = match self.mode { 0 => 6, 1 => 20, 3 => 100, _ => 2 };
            for i in 0..3 {
                self.vals[i] = (sys.rand_n(sides) + 1) as u8;
            }
            self.coin = sys.rand_n(2) == 1;
            self.rnd = sys.rand_n(100) + 1;
            if self.anim % 2 == 0 {
                let f = 900 + sys.rand_n(900) as u16;
                sys.tone(f);
            }
            if self.anim == 0 {
                sys.silence();
                sys.snd(Snd::Accept);
            }
        }
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        gfx::title(a, Self::MODES[self.mode as usize]);
        match self.mode {
            0 => {
                let n = self.n as usize;
                let tot: u32 = self.vals[..n].iter().map(|&v| v as u32).sum();
                gfx::text_c(a, 64, 26, &format!("Total {}", tot), F::XL9x15, true);
                gfx::hint(a, "5:roll A:mode B:count");
                for i in 0..n {
                    let sz = if n == 1 { 44 } else if n == 2 { 38 } else { 36 };
                    let gap = (128 - sz * n as i32) / (n as i32 + 1);
                    die(b, gap + i as i32 * (sz + gap), 10, sz, self.vals[i]);
                }
            }
            1 | 3 => {
                let v = if self.mode == 1 { self.vals[0] as u32 } else { self.rnd };
                gfx::text_c(a, 64, 22, &format!("{}", v), F::H10x20, true);
                gfx::hint(a, "5:roll A:mode");
                gfx::text_c(b, 64, 20, &format!("{}", v), F::H10x20, true);
                b.circle(64, 30, 28, true);
            }
            _ => {
                gfx::text_c(a, 64, 24, if self.coin { "TAILS" } else { "HEADS" }, F::H10x20, true);
                gfx::hint(a, "5:flip A:mode");
                b.circle(64, 32, 28, true);
                b.circle(64, 32, 24, true);
                gfx::text_c(b, 64, 24, if self.coin { "T" } else { "H" }, F::H10x20, true);
            }
        }
    }
}

// ───────────────────────────── System info ─────────────────────────────
pub struct SysInfo {
    page: u8,
    hist: [u8; 64],
    last: u64,
}
impl SysInfo {
    pub fn new() -> Self {
        Self { page: 0, hist: [0; 64], last: 0 }
    }
}
impl App for SysInfo {
    fn key(&mut self, ev: KeyEvent, _sys: &mut Sys) -> Flow {
        if ev.down() {
            match ev.key.nav() {
                Some(crate::input::Nav::Down) | Some(crate::input::Nav::Right) => self.page = (self.page + 1) % 3,
                Some(crate::input::Nav::Up) | Some(crate::input::Nav::Left) => self.page = (self.page + 2) % 3,
                Some(crate::input::Nav::Back) => return Flow::Exit,
                _ => {}
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if sys.now - self.last >= 1000 {
            self.last = sys.now;
            let tot = (sys.info.heap_used + sys.info.heap_free).max(1);
            let pct = (sys.info.heap_free * 100 / tot) as u8;
            self.hist.rotate_left(1);
            self.hist[63] = pct;
        }
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "SYSTEM");
        let up = sys.now / 1000;
        let tot = sys.info.heap_used + sys.info.heap_free;
        let l: Vec<String> = match self.page {
            0 => alloc::vec![
                String::from("Aetheris OS 1.0"),
                String::from("ESP32-C6 RISC-V 160MHz"),
                format!("Flash {} MB  slot {}", sys.info.flash_mb, sys.info.ota_slot),
                format!("MAC {:02X}{:02X}{:02X}{:02X}", sys.wifi.mac[2], sys.wifi.mac[3], sys.wifi.mac[4], sys.wifi.mac[5]),
            ],
            1 => alloc::vec![
                format!("Heap used {} KB", sys.info.heap_used / 1024),
                format!("Heap free {} KB", sys.info.heap_free / 1024),
                format!("Total     {} KB", tot / 1024),
                format!("Uptime {:02}:{:02}:{:02}", up / 3600, up / 60 % 60, up % 60),
            ],
            _ => alloc::vec![
                format!("UI {} fps", sys.info.fps),
                format!("OLED A:{} B:{}", if sys.info.oled_a_ok { "ok" } else { "--" }, if sys.info.oled_b_ok { "ok" } else { "--" }),
                format!("Storage {}", if sys.info.storage_ok { "ok" } else { "none" }),
                format!("IR {}", if sys.ir.blocked { "BLOCKED" } else { "clear" }),
            ],
        };
        for (i, s) in l.iter().enumerate() {
            gfx::text(a, 3, 18 + i as i32 * 9, &gfx::clip(s, 21), F::S5x8, true);
        }
        gfx::hint(a, "up/down:page   *:back");
        aux_panel(b, "HEAP FREE %", &[]);
        for (x, &v) in self.hist.iter().enumerate() {
            let h = v as i32 * 40 / 100;
            b.vline(x as i32 * 2, 58 - h, h.max(1), true);
            b.vline(x as i32 * 2 + 1, 58 - h, h.max(1), true);
        }
        gfx::text_r(b, 126, 14, &format!("{}%", self.hist[63]), F::S5x8, true);
    }
}

// ───────────────────────────── Key test ─────────────────────────────
pub struct KeyTest {
    last: Option<(Key, &'static str)>,
    count: u32,
}
impl KeyTest {
    pub fn new() -> Self {
        Self { last: None, count: 0 }
    }
}
impl App for KeyTest {
    fn key(&mut self, ev: KeyEvent, _sys: &mut Sys) -> Flow {
        use crate::input::Kind::*;
        let k = match ev.kind {
            Press => "press",
            Long => "long",
            Release => "release",
            Repeat => return Flow::Stay,
        };
        if ev.kind == Press {
            self.count += 1;
        }
        self.last = Some((ev.key, k));
        Flow::Stay
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "KEY TEST");
        if let Some((k, kind)) = self.last {
            let mut buf = [0u8; 4];
            gfx::text_c(a, 30, 22, k.ch().encode_utf8(&mut buf), F::H10x20, true);
            gfx::text(a, 60, 22, kind, F::M6x10, true);
            for r in 0..4 {
                for c in 0..4 {
                    if crate::input::KEYMAP[r][c] == k {
                        gfx::text(a, 60, 34, &format!("R{} C{}", r + 1, c + 1), F::M6x10, true);
                    }
                }
            }
        }
        gfx::hint(a, &format!("n={}  hold * to exit", self.count));
        aux_panel(b, "LIVE MATRIX", &[]);
        for r in 0..4i32 {
            for c in 0..4i32 {
                let on = sys.raw_keys & (1 << (r * 4 + c)) != 0;
                let (x, y) = (10 + c * 30, 15 + r * 12);
                if on {
                    b.fill_rect(x, y, 26, 10, true);
                } else {
                    b.rect(x, y, 26, 10, true);
                }
                let mut buf = [0u8; 4];
                let k = crate::input::KEYMAP[r as usize][c as usize];
                gfx::text_c(b, x + 13, y + 1, k.ch().encode_utf8(&mut buf), F::S5x8, !on);
            }
        }
    }
}

// ───────────────────────────── Torch ─────────────────────────────
pub struct Torch {
    mode: u8,
}
impl Torch {
    pub fn new() -> Self {
        Self { mode: 0 }
    }
}
impl App for Torch {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.press() {
            match ev.key {
                Key::D | Key::Star => return Flow::Exit,
                _ => {
                    self.mode = (self.mode + 1) % 4;
                    sys.snd(Snd::Click);
                }
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        sys.led_pulse(120);
    }
    fn keep_awake(&self) -> bool {
        true
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, _sys: &Sys) {
        match self.mode {
            0 => {
                a.fill();
                b.fill();
            }
            1 => a.fill(),
            2 => b.fill(),
            _ => {
                for y in 0..64 {
                    for x in 0..128 {
                        let on = ((x / 4) + (y / 4)) % 2 == 0;
                        a.set(x, y, on);
                        b.set(x, y, !on);
                    }
                }
            }
        }
        if self.mode == 1 {
            gfx::text_inv(b, 20, 28, "any key: next mode", F::M6x10);
        }
        if self.mode == 2 {
            gfx::text_inv(a, 20, 28, "any key: next mode", F::M6x10);
        }
    }
}
