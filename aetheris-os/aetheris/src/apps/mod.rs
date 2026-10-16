//! Aetheris apps. Every app draws on two screens: `a` = main OLED (yellow title band + blue body),
//! `b` = secondary OLED (context panel / visualiser).
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::input::KeyEvent;
use crate::sys::Sys;
use alloc::boxed::Box;
use alloc::string::String;

pub mod games;
pub mod ir;
pub mod music;
pub mod net;
pub mod settings;
pub mod tools;
pub mod widgets;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Stay,
    Exit,
}

pub trait App {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow;
    fn tick(&mut self, _sys: &mut Sys) {}
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys);
    /// Apps that use `*` as a normal key return true so a long-press does not exit.
    fn owns_star(&self) -> bool {
        false
    }
    fn on_exit(&mut self, _sys: &mut Sys) {}
    /// True if the app wants to prevent the idle screen-saver (games, stopwatch, ...).
    fn keep_awake(&self) -> bool {
        false
    }
}

pub struct AppInfo {
    pub name: &'static str,
    pub desc: &'static str,
    pub icon: u8,
}

macro_rules! apps {
    ($( $n:expr, $d:expr, $i:expr, $ctor:expr; )*) => {
        pub const APPS: &[AppInfo] = &[ $( AppInfo { name: $n, desc: $d, icon: $i } ),* ];
        pub fn create(id: usize) -> Box<dyn App> {
            let mut n = 0usize;
            $( if id == n { return Box::new($ctor); } n += 1; )*
            let _ = n;
            Box::new(tools::Torch::new())
        }
    };
}

apps! {
    "Settings", "System preferences", 0, settings::SettingsApp::new();
    "Wi-Fi", "Scan & connect", 1, net::WifiScan::new();
    "Piano", "Notes, # and b, chords", 2, music::Piano::new();
    "Music Box", "Built-in melodies", 3, music::MusicBox::new();
    "Calculator", "Four functions", 4, tools::Calc::new();
    "Clock", "Digital + analog", 5, tools::ClockApp::new();
    "Stopwatch", "Laps to 99:59.9", 6, tools::Stopwatch::new();
    "Timer", "Countdown alarm", 7, tools::Timer::new();
    "Notes", "Type with multi-tap", 8, tools::Notes::new();
    "Snake", "Classic arcade", 9, games::Snake::new();
    "Pong", "You vs the CPU", 10, games::Pong::new();
    "Simon", "Memory + tones", 11, games::Simon::new();
    "Reaction", "Test your reflexes", 12, games::Reaction::new();
    "Dice", "Dice, coin, random", 13, tools::Dice::new();
    "Metronome", "BPM click + LED", 14, music::Metronome::new();
    "Tone Lab", "Buzzer generator", 15, music::ToneLab::new();
    "Morse", "Type & transmit", 16, music::Morse::new();
    "IR Monitor", "Sensor + remote codes", 17, ir::IrMonitor::new();
    "IR Counter", "Count objects", 18, ir::IrCounter::new();
    "Tachometer", "RPM via IR", 19, ir::Tach::new();
    "Matrix", "Digital rain", 20, games::MatrixApp::new();
    "Life", "Conway automaton", 21, games::Life::new();
    "Signal", "Wi-Fi RSSI meter", 22, net::Signal::new();
    "System", "Memory & hardware", 23, tools::SysInfo::new();
    "Key Test", "Matrix scanner", 24, tools::KeyTest::new();
    "Torch", "Screens + LED light", 25, tools::Torch::new();
    "OTA / Programmer", "Self update or flash ESP slave", 26, net::OtaApp::new();
    "Focus", "Pomodoro work timer", 28, tools::Focus::new();
    "Calendar", "Month view + dates", 29, tools::Calendar::new();
    "Converter", "Length, temp, mass...", 30, tools::Converter::new();
    "Playlist", "Build & play queues", 31, music::Playlist::new();
    "Prox Alarm", "IR tripwire siren", 32, ir::ProxAlarm::new();
    "Ninja Run", "Jump the obstacles", 33, games::Ninja::new();
}

/// Scrolling list drawn on the main screen body (y = 17..63). `rows` rows of 11 px.
pub fn list(a: &mut Fb, items: &[(String, String)], sel: usize, rows: usize) {
    let n = items.len();
    if n == 0 {
        return;
    }
    let top = if sel < rows / 2 { 0 } else { (sel - rows / 2).min(n.saturating_sub(rows)) };
    for r in 0..rows {
        let i = top + r;
        if i >= n {
            break;
        }
        let y = 18 + r as i32 * 11;
        let (l, v) = &items[i];
        let on = i == sel;
        if on {
            a.fill_rect(0, y - 1, 123, 11, true);
        }
        let vw = gfx::text_w(v, F::M6x10);
        let maxl = ((120 - vw - 4) / 6).max(3) as usize;
        gfx::text(a, 3, y, &gfx::clip(l, maxl), F::M6x10, !on);
        if !v.is_empty() {
            gfx::text_r(a, 121, y, v, F::M6x10, !on);
        }
    }
    if n > rows {
        let h = 46;
        a.vline(126, 17, h, true);
        let th = (h * rows as i32 / n as i32).max(4);
        let ty = 17 + (h - th) * sel as i32 / (n as i32 - 1).max(1);
        a.fill_rect(125, ty, 3, th, true);
    }
}

/// Context panel on the secondary screen: title + a few text lines.
pub fn aux_panel(b: &mut Fb, title: &str, lines: &[&str]) {
    b.fill_rect(0, 0, 128, 12, true);
    gfx::text_c(b, 64, 1, &gfx::clip(title, 21), F::M6x10, false);
    for (i, l) in lines.iter().enumerate() {
        gfx::text(b, 3, 16 + i as i32 * 10, &gfx::clip(l, 20), F::M6x10, true);
    }
}

/// Format milliseconds as MM:SS.t
pub fn fmt_ms(ms: u64) -> String {
    let t = ms / 100 % 10;
    let s = ms / 1000 % 60;
    let m = ms / 60000;
    alloc::format!("{:02}:{:02}.{}", m, s, t)
}
