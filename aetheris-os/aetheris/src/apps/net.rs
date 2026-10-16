//! Wi-Fi apps: scanner, signal meter, OTA update.
use super::{App, Flow, aux_panel, list};
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::input::{Key, KeyEvent, Nav};
use crate::sound::Snd;
use crate::sys::{OtaState, ProgramState, ProgramTarget, Sys, WifiCmd, WifiStatus, rssi_bars};
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub struct WifiScan {
    sel: usize,
    seq: u32,
    started: bool,
}
impl WifiScan {
    pub fn new() -> Self {
        Self { sel: 0, seq: u32::MAX, started: false }
    }
}
impl App for WifiScan {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !self.started {
            self.started = true;
            sys.wifi_cmd(WifiCmd::Scan);
        }
        if !ev.down() {
            return Flow::Stay;
        }
        match ev.key.nav() {
            Some(Nav::Up) => self.sel = self.sel.saturating_sub(1),
            Some(Nav::Down) => self.sel = (self.sel + 1).min(sys.wifi.aps.len().saturating_sub(1)),
            Some(Nav::Left) | Some(Nav::Right) if ev.press() => {
                sys.wifi_cmd(WifiCmd::Scan);
                sys.snd(Snd::Select);
            }
            Some(Nav::Back) if ev.press() => return Flow::Exit,
            _ => {}
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if !self.started {
            self.started = true;
            sys.wifi_cmd(WifiCmd::Scan);
        }
        if sys.wifi.scan_seq != self.seq {
            self.seq = sys.wifi.scan_seq;
            if !sys.wifi.aps.is_empty() {
                sys.snd(Snd::Notify);
            }
        }
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "WI-FI");
        if sys.wifi.status == WifiStatus::Scanning {
            gfx::text_c(a, 64, 32, "scanning...", F::L7x13, true);
            let x = (sys.now / 20 % 100) as i32;
            a.fill_rect(14 + x, 48, 10, 3, true);
        } else if sys.wifi.status == WifiStatus::Off {
            gfx::text_c(a, 64, 28, "Wi-Fi is off", F::L7x13, true);
        } else if sys.wifi.aps.is_empty() {
            gfx::text_c(a, 64, 28, "no networks found", F::M6x10, true);
            gfx::text_c(a, 64, 42, "<>: rescan", F::M6x10, true);
        } else {
            let items: Vec<(String, String)> = sys.wifi.aps.iter().map(|ap| (ap.ssid.clone(), format!("{}", ap.rssi))).collect();
            list(a, &items, self.sel.min(items.len() - 1), 4);
        }
        aux_panel(b, "NETWORK", &[]);
        if let Some(ap) = sys.wifi.aps.get(self.sel) {
            gfx::text(b, 3, 15, &gfx::clip(&ap.ssid, 21), F::M6x10, true);
            gfx::text(b, 3, 27, &format!("{} dBm  ch {}", ap.rssi, ap.ch), F::M6x10, true);
            gfx::text(b, 3, 39, if ap.open { "open network" } else { "secured (WPA)" }, F::M6x10, true);
            gfx::wifi_icon(b, 108, 38, rssi_bars(ap.rssi), true);
            gfx::text(b, 3, 52, "join: Settings > Wi-Fi", F::S5x8, true);
        } else {
            gfx::text_c(b, 64, 30, "up/down select", F::M6x10, true);
        }
    }
}

pub struct Signal {
    hist: [i8; 64],
    last: u64,
    beep: bool,
    next_beep: u64,
}
impl Signal {
    pub fn new() -> Self {
        Self { hist: [-100; 64], last: 0, beep: false, next_beep: 0 }
    }
}
impl App for Signal {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if ev.press() {
            match ev.key {
                Key::Hash | Key::K5 => {
                    self.beep = !self.beep;
                    sys.snd(Snd::Select);
                }
                Key::Star | Key::D => return Flow::Exit,
                _ => {}
            }
        }
        Flow::Stay
    }
    fn tick(&mut self, sys: &mut Sys) {
        if sys.now - self.last >= 500 {
            self.last = sys.now;
            self.hist.rotate_left(1);
            self.hist[63] = if sys.wifi.connected() { sys.wifi.rssi } else { -100 };
        }
        // Geiger-style "find the router": faster clicks = stronger signal.
        if self.beep && sys.wifi.connected() && sys.now >= self.next_beep {
            let strength = (sys.wifi.rssi as i32 + 100).clamp(1, 60) as u64;
            self.next_beep = sys.now + 700 - strength * 10;
            sys.play(alloc::vec![crate::sound::t(1000 + strength as u16 * 25, 20)]);
        }
    }
    fn keep_awake(&self) -> bool {
        self.beep
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, "SIGNAL");
        if sys.wifi.connected() {
            gfx::text(a, 4, 19, &format!("{} dBm", sys.wifi.rssi), F::H10x20, true);
            gfx::text(a, 4, 41, &gfx::clip(&sys.wifi.ssid, 20), F::M6x10, true);
        } else {
            gfx::text_c(a, 64, 28, "not connected", F::L7x13, true);
        }
        gfx::hint(a, if self.beep { "#:beeper ON (faster=near)" } else { "#:geiger beeper (off)" });
        aux_panel(b, "RSSI HISTORY", &[]);
        for (x, &v) in self.hist.iter().enumerate() {
            let h = ((v as i32 + 100) * 44 / 70).clamp(1, 44);
            b.vline(x as i32 * 2, 61 - h, h, true);
            b.vline(x as i32 * 2 + 1, 61 - h, h, true);
        }
    }
}

pub struct OtaApp {
    on: bool,
    slave: bool,
    target: usize,
}
impl OtaApp {
    pub fn new() -> Self {
        Self { on: false, slave: false, target: 0 }
    }
}
impl App for OtaApp {
    fn key(&mut self, ev: KeyEvent, sys: &mut Sys) -> Flow {
        if !ev.press() {
            return Flow::Stay;
        }
        match ev.key {
            Key::K2 | Key::K8 => {
                if self.on {
                    sys.snd(Snd::Error);
                } else {
                    self.slave = !self.slave;
                    sys.snd(Snd::Select);
                }
            }
            Key::K4 | Key::K6 if self.slave => {
                if self.on {
                    sys.snd(Snd::Error);
                } else {
                    if ev.key == Key::K4 {
                        self.target = self.target.saturating_sub(1);
                    } else {
                        self.target = (self.target + 1).min(ProgramTarget::ALL.len() - 1);
                    }
                    sys.snd(Snd::Select);
                }
            }
            Key::Hash | Key::K5 => {
                if sys.wifi.connected() {
                    self.on = !self.on;
                    if self.slave {
                        sys.wifi_cmd(WifiCmd::ProgramServer { on: self.on, target: ProgramTarget::ALL[self.target] });
                    } else {
                        sys.wifi_cmd(WifiCmd::OtaServer(self.on));
                    }
                    sys.snd(Snd::Accept);
                } else {
                    sys.snd(Snd::Error);
                }
            }
            Key::Star | Key::D => {
                if self.on {
                    if self.slave {
                        sys.wifi_cmd(WifiCmd::ProgramServer { on: false, target: ProgramTarget::ALL[self.target] });
                    } else {
                        sys.wifi_cmd(WifiCmd::OtaServer(false));
                    }
                    self.on = false;
                }
                return Flow::Exit;
            }
            _ => {}
        }
        Flow::Stay
    }
    fn keep_awake(&self) -> bool {
        self.on
    }
    fn draw(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        gfx::title(a, if self.slave { "SLAVE FLASH" } else { "SELF OTA" });
        if !sys.wifi.connected() {
            gfx::text_c(a, 64, 26, "Connect to Wi-Fi first", F::M6x10, true);
            gfx::text_c(a, 64, 40, "Settings > Wi-Fi", F::M6x10, true);
        } else {
            let ip = sys.wifi.ip;
            gfx::text(a, 4, 19, &format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]), F::M6x10, true);
            let st = if self.slave {
                match &sys.wifi.program {
                    ProgramState::Idle => String::from("server off"),
                    ProgramState::Listening => String::from("waiting for bin"),
                    ProgramState::Staging { done, total } => format!("stage {} / {} KB", done / 1024, total / 1024),
                    ProgramState::Connecting => String::from("bootloader sync"),
                    ProgramState::Flashing { done, total } => format!("flash {} / {} KB", done / 1024, total / 1024),
                    ProgramState::Done => String::from("done; slave rebooted"),
                    ProgramState::Failed(e) => format!("failed: {}", e),
                }
            } else {
                match &sys.wifi.ota {
                    OtaState::Idle => String::from("server off"),
                    OtaState::Listening => String::from("waiting for upload"),
                    OtaState::Receiving { done, total } => format!("{} / {} KB", done / 1024, total / 1024),
                    OtaState::Done => String::from("done! rebooting"),
                    OtaState::Failed(e) => format!("failed: {}", e),
                }
            };
            gfx::text(a, 4, 32, &gfx::clip(&st, 20), F::M6x10, true);
            if self.slave {
                gfx::text(a, 4, 44, ProgramTarget::ALL[self.target].label(), F::S5x8, true);
            } else if let OtaState::Receiving { done, total } = sys.wifi.ota {
                gfx::bar(a, 4, 44, 120, 8, done as i32 / 64, (total as i32 / 64).max(1));
            }
        }
        gfx::hint(a, "2/8 mode  4/6 chip  #:server");
        if self.slave {
            aux_panel(b, "SLAVE PROGRAM", &[]);
            gfx::text(b, 3, 15, "app-only bin, offset 0x10000", F::S5x8, true);
            gfx::text(b, 3, 25, "connect UART/BOOT/RESET/GND", F::S5x8, true);
            let path = format!("POST bin /program/{}", ProgramTarget::ALL[self.target].slug());
            gfx::text(b, 3, 35, &path, F::S5x8, true);
            gfx::text(b, 3, 45, "checks chip id before flash", F::S5x8, true);
            gfx::text(b, 3, 55, "* exits; may need recovery", F::S5x8, true);
        } else {
            aux_panel(b, "HOW TO UPDATE", &[]);
            gfx::text(b, 3, 15, "1 espflash save-image", F::S5x8, true);
            gfx::text(b, 3, 25, "2 --chip esp32c6", F::S5x8, true);
            gfx::text(b, 3, 35, " (no --merge)", F::S5x8, true);
            gfx::text(b, 3, 45, "3 curl --data-binary", F::S5x8, true);
            gfx::text(b, 3, 55, " @a.bin http://IP/ota", F::S5x8, true);
        }
    }
}
