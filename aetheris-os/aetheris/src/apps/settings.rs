//! Settings: a small phone-style settings app (Wi-Fi, display, sound, security, time, IR, system).
use super::widgets::{Res, TextPrompt};
use super::{App, Flow, aux_panel, list};
use crate::clock;
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::icons;
use crate::input::{KeyEvent, Mode, Nav};
use crate::settings::Settings;
use crate::sound::Snd;
use crate::sys::{Sys, WifiCmd, WifiStatus};
use crate::wallpaper::{self, Wall};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Root,
    Wifi,
    Pick,
    Display,
    Sound,
    Security,
    Time,
    Ir,
    System,
    About,
}

enum Prompt {
    WifiPass(String),
    User,
    NewPass,
    RepeatPass(String),
    Time,
    Date,
}

const SLEEP: [u16; 6] = [0, 30, 60, 120, 300, 600];
const LOCK: [u16; 5] = [0, 60, 300, 900, 1800];

fn secs(v: u16) -> String {
    match v {
        0 => String::from("never"),
        s if s < 60 => format!("{}s", s),
        s => format!("{}m", s / 60),
    }
}
fn cycle(list: &[u16], cur: u16, dir: i32) -> u16 {
    let i = list.iter().position(|&v| v == cur).unwrap_or(0) as i32;
    list[(i + dir).rem_euclid(list.len() as i32) as usize]
}
fn onoff(b: bool) -> String {
    String::from(if b { "ON" } else { "off" })
}
fn bars(v: u8, max: u8) -> String {
    let mut s = String::new();
    for i in 0..max {
        s.push(if i < v { '|' } else { '.' });
    }
    s
}

pub struct SettingsApp {
    page: Page,
    sel: [usize; 10],
    prompt: Option<(Prompt, TextPrompt)>,
    msg: Option<(String, u64)>,
    wall: Wall,
    confirm_until: u64,
    scan_seq: u32,
}

impl SettingsApp {
    pub fn new() -> Self {
        Self { page: Page::Root, sel: [0; 10], prompt: None, msg: None, wall: Wall::new(0, 0x5E1E57E), confirm_until: 0, scan_seq: 0 }
    }
    fn idx(p: Page) -> usize {
        p as usize
    }
    fn cur(&self) -> usize {
        self.sel[Self::idx(self.page)]
    }
    fn title(&self) -> &'static str {
        match self.page {
            Page::Root => "SETTINGS",
            Page::Wifi => "Wi-Fi",
            Page::Pick => "NETWORKS",
            Page::Display => "DISPLAY",
            Page::Sound => "SOUND",
            Page::Security => "SECURITY",
            Page::Time => "TIME",
            Page::Ir => "IR SENSOR",
            Page::System => "SYSTEM",
            Page::About => "ABOUT",
        }
    }
    fn toast(&mut self, s: &str, sys: &Sys) {
        self.msg = Some((String::from(s), sys.now + 1800));
    }

    fn wifi_status(sys: &Sys) -> String {
        match sys.wifi.status {
            WifiStatus::Off => String::from("off"),
            WifiStatus::Idle => String::from("idle"),
            WifiStatus::Scanning => String::from("scanning"),
            WifiStatus::Connecting => String::from("connecting"),
            WifiStatus::Connected => gfx::clip(&sys.wifi.ssid, 12),
            WifiStatus::Failed => String::from("failed"),
        }
    }

    fn rows(&self, sys: &Sys) -> Vec<(String, String)> {
        let s = &sys.settings;
        let r = |a: &str, b: String| (String::from(a), b);
        match self.page {
            Page::Root => alloc::vec![
                r("Wi-Fi", Self::wifi_status(sys)),
                r("Display", String::new()),
                r("Sound", bars(s.volume, 5)),
                r("Security", String::new()),
                r("Date & Time", String::new()),
                r("IR Sensor", onoff(s.ir_wake)),
                r("System", String::new()),
                r("About", String::new()),
            ],
            Page::Wifi => alloc::vec![
                r("Wi-Fi radio", onoff(sys.wifi.status != WifiStatus::Off)),
                r("Scan networks", String::from(">")),
                r("Status", Self::wifi_status(sys)),
                r("Saved network", gfx::clip(&s.wifi_ssid, 10)),
                r("Auto-connect", onoff(s.wifi_auto)),
                r("Forget saved", String::new()),
                r("Sync clock", if sys.wifi.time_synced { String::from("done") } else { String::from(">") }),
                r("IP", if sys.wifi.connected() { format!("{}.{}.{}.{}", sys.wifi.ip[0], sys.wifi.ip[1], sys.wifi.ip[2], sys.wifi.ip[3]) } else { String::from("--") }),
            ],
            Page::Pick => {
                let mut v: Vec<(String, String)> = sys.wifi.aps.iter().map(|ap| (ap.ssid.clone(), format!("{}{}", if ap.open { "" } else { "*" }, ap.rssi))).collect();
                if v.is_empty() {
                    v.push(r(if sys.wifi.status == WifiStatus::Scanning { "scanning..." } else { "none - <> rescan" }, String::new()));
                }
                v
            }
            Page::Display => alloc::vec![
                r("Wallpaper A", String::from(wallpaper::NAMES[(s.wp_a % wallpaper::COUNT) as usize])),
                r("Wallpaper B", String::from(wallpaper::NAMES[(s.wp_b % wallpaper::COUNT) as usize])),
                r("Brightness A", bars(s.bright_a, 4)),
                r("Brightness B", bars(s.bright_b, 4)),
                r("Invert A", onoff(s.invert_a)),
                r("Invert B", onoff(s.invert_b)),
                r("Screen off", secs(s.sleep_s)),
                r("Auto-lock", secs(s.lock_s)),
            ],
            Page::Sound => alloc::vec![
                r("Volume", bars(s.volume, 5)),
                r("Key click", onoff(s.key_click)),
                r("Boot sound", onoff(s.boot_sound)),
                r("Test: accept", String::new()),
                r("Test: error", String::new()),
                r("Test: alert", String::new()),
                r("Test: boot", String::new()),
            ],
            Page::Security => alloc::vec![
                r("Username", gfx::clip(&s.user, 10)),
                r("Change password", String::new()),
                r("Auto-lock", secs(s.lock_s)),
                r("Lock now", String::new()),
            ],
            Page::Time => {
                let t = sys.local_time();
                alloc::vec![
                    r("Time", format!("{:02}:{:02}:{:02}", t.h, t.mi, t.s)),
                    r("Date", format!("{:02}/{:02}/{}", t.d, t.mo, t.y % 100)),
                    r("Time zone", format!("UTC{:+}:{:02}", s.tz_min / 60, (s.tz_min % 60).abs())),
                    r("Sync via Wi-Fi", if sys.wifi.time_synced { String::from("done") } else { String::from(">") }),
                ]
            }
            Page::Ir => alloc::vec![
                r("Wake on IR", onoff(s.ir_wake)),
                r("Sensor", String::from(if sys.ir.blocked { "BLOCKED" } else { "clear" })),
                r("Events", format!("{}", sys.ir.falls)),
            ],
            Page::System => alloc::vec![
                r("Free heap", format!("{}K", sys.info.heap_free / 1024)),
                r("Storage", String::from(if sys.info.storage_ok { "ok" } else { "none" })),
                r("Sleep screens", String::new()),
                r("Reboot", String::new()),
                r("Factory reset", if self.confirm_until > sys.now { String::from("sure? #") } else { String::new() }),
            ],
            Page::About => alloc::vec![
                r("Aetheris OS", String::from("v1.0")),
                r("By", String::from("Seleste")),
                r("MCU", String::from("ESP32-C6")),
                r("Flash", format!("{} MB", sys.info.flash_mb)),
                r("Slot", String::from(sys.info.ota_slot)),
            ],
        }
    }

    fn set_clock(&mut self, sys: &mut Sys, h: u8, mi: u8, y: i32, mo: u8, d: u8, s: u8) {
        let local = clock::to_epoch(y, mo, d, h, mi, s);
        sys.settings.epoch_off = local - sys.settings.tz_min as i64 * 60 - (sys.now / 1000) as i64;
        sys.save();
    }

    fn act(&mut self, i: usize, dir: i32, press: bool, sys: &mut Sys) {
        let ok = dir == 0;
        match self.page {
            Page::Root => {
                if ok && press {
                    self.page = [Page::Wifi, Page::Display, Page::Sound, Page::Security, Page::Time, Page::Ir, Page::System, Page::About][i.min(7)];
                    sys.snd(Snd::Select);
                    if self.page == Page::Ir {
                        // handled below via Ir page
                    }
                }
            }
            Page::Wifi => match i {
                0 if ok && press => {
                    let on = sys.wifi.status == WifiStatus::Off;
                    sys.wifi_cmd(WifiCmd::SetEnabled(on));
                    sys.snd(Snd::Select);
                }
                1 if ok && press => {
                    self.page = Page::Pick;
                    sys.wifi_cmd(WifiCmd::Scan);
                    sys.snd(Snd::Select);
                }
                2 if ok && press => {
                    if sys.wifi.connected() {
                        sys.wifi_cmd(WifiCmd::Disconnect);
                    }
                }
                3 if ok && press => {
                    if !sys.settings.wifi_ssid.is_empty() {
                        let (s, p) = (sys.settings.wifi_ssid.clone(), sys.settings.wifi_pass.clone());
                        sys.wifi_cmd(WifiCmd::Connect { ssid: s, pass: p });
                        sys.snd(Snd::Select);
                    } else {
                        sys.snd(Snd::Error);
                    }
                }
                4 => {
                    sys.settings.wifi_auto = !sys.settings.wifi_auto;
                    sys.save();
                    sys.snd(Snd::Click);
                }
                5 if ok && press => {
                    sys.settings.wifi_ssid.clear();
                    sys.settings.wifi_pass.clear();
                    sys.save();
                    sys.snd(Snd::Back);
                    self.toast("saved network removed", sys);
                }
                6 if ok && press => {
                    if sys.wifi.connected() {
                        sys.wifi_cmd(WifiCmd::SyncTime);
                        self.toast("syncing time...", sys);
                    } else {
                        sys.snd(Snd::Error);
                    }
                }
                _ => {}
            },
            Page::Pick => {
                if dir != 0 {
                    sys.wifi_cmd(WifiCmd::Scan);
                    sys.snd(Snd::Select);
                } else if press && let Some(ap) = sys.wifi.aps.get(i).cloned() {
                    if ap.open {
                        sys.settings.wifi_ssid = ap.ssid.clone();
                        sys.settings.wifi_pass.clear();
                        sys.save();
                        sys.wifi_cmd(WifiCmd::Connect { ssid: ap.ssid, pass: String::new() });
                        self.page = Page::Wifi;
                    } else {
                        let mut p = TextPrompt::new(&format!("Password: {}", gfx::clip(&ap.ssid, 9)), 63, true, "");
                        p.entry.mode = Mode::Lower;
                        self.prompt = Some((Prompt::WifiPass(ap.ssid), p));
                    }
                    sys.snd(Snd::Select);
                }
            }
            Page::Display => {
                let d = if dir == 0 { 1 } else { dir };
                let s = &mut sys.settings;
                match i {
                    0 => s.wp_a = (s.wp_a as i32 + d).rem_euclid(wallpaper::COUNT as i32) as u8,
                    1 => s.wp_b = (s.wp_b as i32 + d).rem_euclid(wallpaper::COUNT as i32) as u8,
                    2 => s.bright_a = (s.bright_a as i32 + d).rem_euclid(5) as u8,
                    3 => s.bright_b = (s.bright_b as i32 + d).rem_euclid(5) as u8,
                    4 => s.invert_a = !s.invert_a,
                    5 => s.invert_b = !s.invert_b,
                    6 => s.sleep_s = cycle(&SLEEP, s.sleep_s, d),
                    _ => s.lock_s = cycle(&LOCK, s.lock_s, d),
                }
                sys.save();
                sys.snd(Snd::Click);
            }
            Page::Sound => {
                let d = if dir == 0 { 1 } else { dir };
                match i {
                    0 => {
                        sys.settings.volume = (sys.settings.volume as i32 + d).rem_euclid(6) as u8;
                        sys.save();
                        sys.snd(Snd::Accept);
                    }
                    1 => {
                        sys.settings.key_click = !sys.settings.key_click;
                        sys.save();
                        sys.snd(Snd::Click);
                    }
                    2 => {
                        sys.settings.boot_sound = !sys.settings.boot_sound;
                        sys.save();
                        sys.snd(Snd::Click);
                    }
                    3 if ok && press => sys.snd(Snd::Accept),
                    4 if ok && press => sys.snd(Snd::Error),
                    5 if ok && press => sys.snd(Snd::Notify),
                    6 if ok && press => sys.snd(Snd::Boot),
                    _ => {}
                }
            }
            Page::Security => match i {
                0 if ok && press => {
                    let mut p = TextPrompt::new("New username", 15, false, &sys.settings.user.clone());
                    p.entry.mode = Mode::Lower;
                    self.prompt = Some((Prompt::User, p));
                }
                1 if ok && press => {
                    self.prompt = Some((Prompt::NewPass, TextPrompt::new("New password", 24, true, "")));
                }
                2 => {
                    let d = if dir == 0 { 1 } else { dir };
                    sys.settings.lock_s = cycle(&LOCK, sys.settings.lock_s, d);
                    sys.save();
                    sys.snd(Snd::Click);
                }
                3 if ok && press => sys.lock_req = true,
                _ => {}
            },
            Page::Time => match i {
                0 if ok && press => {
                    let mut p = TextPrompt::new("Time as HHMM", 4, false, "");
                    p.entry.mode = Mode::Digits;
                    self.prompt = Some((Prompt::Time, p));
                }
                1 if ok && press => {
                    let mut p = TextPrompt::new("Date as YYYYMMDD", 8, false, "");
                    p.entry.mode = Mode::Digits;
                    self.prompt = Some((Prompt::Date, p));
                }
                2 => {
                    let d = if dir == 0 { 1 } else { dir };
                    let t = (sys.settings.tz_min as i32 + d * 30).clamp(-12 * 60, 14 * 60);
                    let old = sys.settings.tz_min;
                    sys.settings.tz_min = t as i16;
                    // keep the displayed local time unchanged: shift the offset the other way
                    sys.settings.epoch_off += (old as i64 - t as i64) * 60;
                    sys.save();
                    sys.snd(Snd::Click);
                }
                3 if ok && press => {
                    if sys.wifi.connected() {
                        sys.wifi_cmd(WifiCmd::SyncTime);
                        self.toast("syncing time...", sys);
                    } else {
                        sys.snd(Snd::Error);
                    }
                }
                _ => {}
            },
            Page::Ir => {
                if i == 0 {
                    sys.settings.ir_wake = !sys.settings.ir_wake;
                    sys.save();
                    sys.snd(Snd::Click);
                }
            }
            Page::System => match i {
                2 if ok && press => sys.fx.push(crate::sys::Effect::Sleep),
                3 if ok && press => {
                    sys.snd(Snd::Shutdown);
                    sys.fx.push(crate::sys::Effect::Reboot);
                }
                4 if ok && press => {
                    if self.confirm_until > sys.now {
                        sys.settings = Settings::default();
                        sys.save();
                        sys.snd(Snd::Shutdown);
                        sys.fx.push(crate::sys::Effect::Reboot);
                    } else {
                        self.confirm_until = sys.now + 4000;
                        sys.snd(Snd::Warn);
                    }
                }
                _ => {}
            },
            Page::About => {}
        }
    }

    fn finish_prompt(&mut self, sys: &mut Sys) {
        let Some((kind, p)) = self.prompt.take() else { return };
        let text = String::from(p.text());
        match kind {
            Prompt::WifiPass(ssid) => {
                sys.settings.wifi_ssid = ssid.clone();
                sys.settings.wifi_pass = text.clone();
                sys.save();
                sys.wifi_cmd(WifiCmd::Connect { ssid, pass: text });
                self.page = Page::Wifi;
                sys.snd(Snd::Accept);
            }
            Prompt::User => {
                if text.len() >= 2 {
                    sys.settings.user = text;
                    sys.save();
                    sys.snd(Snd::Accept);
                    self.toast("username changed", sys);
                } else {
                    sys.snd(Snd::Error);
                }
            }
            Prompt::NewPass => {
                if text.len() >= 4 {
                    self.prompt = Some((Prompt::RepeatPass(text), TextPrompt::new("Repeat password", 24, true, "")));
                    sys.snd(Snd::Select);
                } else {
                    self.toast("min 4 characters", sys);
                    sys.snd(Snd::Error);
                }
            }
            Prompt::RepeatPass(first) => {
                if first == text {
                    let seed = sys.rand();
                    sys.settings.set_password(&text, seed);
                    sys.save();
                    sys.snd(Snd::Accept);
                    self.toast("password changed", sys);
                } else {
                    sys.snd(Snd::Error);
                    self.toast("did not match", sys);
                }
            }
            Prompt::Time => {
                let v: u32 = text.parse().unwrap_or(9999);
                let (h, m) = ((v / 100) as u8, (v % 100) as u8);
                if text.len() == 4 && h < 24 && m < 60 {
                    let t = sys.local_time();
                    self.set_clock(sys, h, m, t.y, t.mo, t.d, 0);
                    sys.snd(Snd::Accept);
                } else {
                    sys.snd(Snd::Error);
                }
            }
            Prompt::Date => {
                let v: u32 = text.parse().unwrap_or(0);
                let (y, mo, d) = ((v / 10000) as i32, ((v / 100) % 100) as u8, (v % 100) as u8);
                if text.len() == 8 && (2000..2100).contains(&y) && (1..=12).contains(&mo) && d >= 1 && d <= clock::days_in_month(y, mo) {
                    let t = sys.local_time();
                    self.set_clock(sys, t.h, t.mi, y, mo, d, t.s);
                    sys.snd(Snd::Accept);
                } else {
                    sys.snd(Snd::Error);
                }
            }
        }
    }
}

impl App for SettingsApp {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if self.prompt.is_some() {
            let res = self.prompt.as_mut().unwrap().1.key(ev, sys);
            match res {
                Res::Submit => self.finish_prompt(sys),
                Res::Cancel => {
                    self.prompt = None;
                    sys.snd(Snd::Back);
                }
                Res::None => {}
            }
            return Flow::Stay;
        }
        if !ev.down() {
            return Flow::Stay;
        }
        let n = self.rows(sys).len();
        let pi = Self::idx(self.page);
        match ev.key.nav() {
            Some(Nav::Up) => self.sel[pi] = (self.sel[pi] + n - 1) % n,
            Some(Nav::Down) => self.sel[pi] = (self.sel[pi] + 1) % n,
            Some(Nav::Left) => {
                let i = self.sel[pi];
                self.act(i, -1, ev.press(), sys)
            }
            Some(Nav::Right) => {
                let i = self.sel[pi];
                self.act(i, 1, ev.press(), sys)
            }
            Some(Nav::Ok) => {
                let i = self.sel[pi];
                self.act(i, 0, ev.press(), sys)
            }
            Some(Nav::Back) if ev.press() => {
                if self.page == Page::Root {
                    return Flow::Exit;
                }
                self.page = if self.page == Page::Pick { Page::Wifi } else { Page::Root };
                sys.snd(Snd::Back);
            }
            _ => {}
        }
        if matches!(ev.key.nav(), Some(Nav::Up) | Some(Nav::Down)) {
            sys.snd(Snd::Click);
        }
        Flow::Stay
    }

    fn tick(&mut self, sys: &mut Sys) {
        if let Some((_, p)) = self.prompt.as_mut() {
            p.tick(sys);
        }
        if self.page == Page::Display {
            let k = if self.cur() == 1 || self.cur() == 5 || self.cur() == 3 { sys.settings.wp_b } else { sys.settings.wp_a };
            self.wall.set_kind(k);
            self.wall.step();
        }
        if let Some((_, t)) = &self.msg
            && sys.now > *t
        {
            self.msg = None;
        }
        if self.page == Page::Pick && sys.wifi.scan_seq != self.scan_seq {
            self.scan_seq = sys.wifi.scan_seq;
            self.sel[Self::idx(Page::Pick)] = 0;
        }
        if self.page == Page::Wifi && sys.wifi.status == WifiStatus::Connected && self.msg.is_none() && sys.wifi.time_synced {
            // nothing: status shown in rows
        }
    }

    fn owns_star(&self) -> bool {
        self.prompt.is_some()
    }

    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        if let Some((kind, p)) = &self.prompt {
            gfx::title(a, match kind {
                Prompt::WifiPass(_) => "WI-FI KEY",
                Prompt::User => "USERNAME",
                Prompt::NewPass | Prompt::RepeatPass(_) => "PASSWORD",
                Prompt::Time => "SET TIME",
                Prompt::Date => "SET DATE",
            });
            p.draw(a, b, sys);
            return;
        }
        gfx::title(a, self.title());
        if self.page == Page::About {
            for (i, (l, v)) in self.rows(sys).iter().enumerate() {
                gfx::text(a, 4, 19 + i as i32 * 9, &format!("{:<8}{}", l, v), F::M6x10, true);
            }
        } else {
            let rows = self.rows(sys);
            let sel = self.cur().min(rows.len().saturating_sub(1));
            list(a, &rows, sel, 4);
        }
        if let Some((m, _)) = &self.msg {
            gfx::text_inv(a, 8, 52, m, F::M6x10);
        }
        // ── second screen ──
        let sel = self.cur();
        match self.page {
            Page::Display => {
                self.wall.draw(b);
                let name = wallpaper::NAMES[(self.wall.kind % wallpaper::COUNT) as usize];
                gfx::text_inv(b, 2, 54, name, F::S5x8);
            }
            Page::Root => {
                let ic = [1u8, 15, 2, 27, 5, 17, 23, 0][sel.min(7)];
                icons::draw(b, ic, 52, 8);
                let names = ["Wi-Fi", "Display", "Sound", "Security", "Date & Time", "IR Sensor", "System", "About"];
                gfx::text_c(b, 64, 40, names[sel.min(7)], F::XL9x15, true);
            }
            Page::Wifi | Page::Pick => {
                gfx::wifi_icon(b, 50, 6, sys.wifi.bars().max(if sys.wifi.connected() { 1 } else { 0 }), sys.wifi.connected());
                b.fill_rect(46, 2, 1, 1, false);
                gfx::text_c(b, 64, 24, &Self::wifi_status(sys), F::L7x13, true);
                if sys.wifi.connected() {
                    gfx::text_c(b, 64, 40, &format!("{} dBm", sys.wifi.rssi), F::M6x10, true);
                    gfx::text_c(b, 64, 52, &format!("{}.{}.{}.{}", sys.wifi.ip[0], sys.wifi.ip[1], sys.wifi.ip[2], sys.wifi.ip[3]), F::M6x10, true);
                } else if !sys.wifi.err.is_empty() {
                    gfx::text_c(b, 64, 44, sys.wifi.err, F::M6x10, true);
                }
            }
            Page::Sound => {
                gfx::speaker_icon(b, 52, 8, sys.settings.volume);
                gfx::bar(b, 14, 34, 100, 10, sys.settings.volume as i32, 5);
                gfx::text_c(b, 64, 50, &format!("volume {}/5", sys.settings.volume), F::M6x10, true);
            }
            Page::Security => {
                icons::draw(b, 27, 52, 6);
                gfx::text_c(b, 64, 38, &gfx::clip(&sys.settings.user, 15), F::L7x13, true);
                gfx::text_c(b, 64, 52, if sys.settings.is_default_login() { "default login!" } else { "password set" }, F::M6x10, true);
            }
            Page::Time => {
                let t = sys.local_time();
                gfx::text_c(b, 64, 8, &format!("{:02}:{:02}:{:02}", t.h, t.mi, t.s), F::H10x20, true);
                gfx::text_c(b, 64, 34, &format!("{} {} {} {}", clock::DAYS[t.wd as usize], t.d, clock::MONTHS[(t.mo - 1) as usize], t.y), F::M6x10, true);
                gfx::text_c(b, 64, 48, if sys.wifi.time_synced { "synced" } else { "manual" }, F::M6x10, true);
            }
            Page::Ir => {
                icons::draw(b, 17, 52, 8);
                gfx::text_c(b, 64, 42, if sys.ir.blocked { "BLOCKED" } else { "CLEAR" }, F::XL9x15, true);
            }
            _ => {
                aux_panel(b, self.title(), &[]);
                icons::draw(b, 23, 52, 20);
            }
        }
    }
}
