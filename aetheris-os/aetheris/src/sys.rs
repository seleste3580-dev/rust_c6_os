//! Shared system context handed to every screen/app. Contains no hardware: apps request side effects
//! through `Effect`s which the firmware main loop applies to the real peripherals.
use crate::settings::Settings;
use crate::sound::{Snd, Step};
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum WifiCmd {
    Scan,
    Connect { ssid: String, pass: String },
    Disconnect,
    SetEnabled(bool),
    OtaServer(bool),
    ProgramServer { on: bool, target: ProgramTarget },
    SyncTime,
}

pub enum Effect {
    Snd(Snd),
    Tone(u16),
    Silence,
    Play(Vec<Step>),
    LedPulse(u16),
    Save,
    Reboot,
    Sleep,
    Wifi(WifiCmd),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WifiStatus {
    Off,
    Idle,
    Scanning,
    Connecting,
    Connected,
    Failed,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApRec {
    pub ssid: String,
    pub rssi: i8,
    pub ch: u8,
    pub open: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OtaState {
    Idle,
    Listening,
    Receiving { done: u32, total: u32 },
    Done,
    Failed(&'static str),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ProgramTarget {
    Esp32C3,
    Esp32S3,
    Esp32C6,
}

impl ProgramTarget {
    pub const ALL: [Self; 3] = [Self::Esp32C3, Self::Esp32S3, Self::Esp32C6];

    pub const fn index(self) -> u8 {
        match self {
            Self::Esp32C3 => 0,
            Self::Esp32S3 => 1,
            Self::Esp32C6 => 2,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Esp32C3 => "ESP32-C3",
            Self::Esp32S3 => "ESP32-S3",
            Self::Esp32C6 => "ESP32-C6",
        }
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Esp32C3 => "c3",
            Self::Esp32S3 => "s3",
            Self::Esp32C6 => "c6",
        }
    }

    pub const fn chip_id(self) -> u16 {
        match self {
            Self::Esp32C3 => 5,
            Self::Esp32S3 => 9,
            Self::Esp32C6 => 13,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum ProgramState {
    Idle,
    Listening,
    Staging { done: u32, total: u32 },
    Connecting,
    Flashing { done: u32, total: u32 },
    Done,
    Failed(&'static str),
}

#[derive(Clone)]
pub struct WifiState {
    pub status: WifiStatus,
    pub ssid: String,
    pub rssi: i8,
    pub ip: [u8; 4],
    pub aps: Vec<ApRec>,
    pub scan_seq: u32,
    pub err: &'static str,
    pub ota: OtaState,
    pub program: ProgramState,
    pub time_synced: bool,
    pub mac: [u8; 6],
}

impl WifiState {
    pub const fn new() -> Self {
        Self {
            status: WifiStatus::Off,
            ssid: String::new(),
            rssi: -100,
            ip: [0; 4],
            aps: Vec::new(),
            scan_seq: 0,
            err: "",
            ota: OtaState::Idle,
            program: ProgramState::Idle,
            time_synced: false,
            mac: [0; 6],
        }
    }
}

impl Default for WifiState {
    fn default() -> Self {
        Self::new()
    }
}

impl WifiState {
    pub fn connected(&self) -> bool {
        self.status == WifiStatus::Connected
    }
    pub fn bars(&self) -> u8 {
        if !self.connected() {
            return 0;
        }
        rssi_bars(self.rssi)
    }
}

pub fn rssi_bars(rssi: i8) -> u8 {
    match rssi {
        r if r >= -60 => 3,
        r if r >= -72 => 2,
        r if r >= -85 => 1,
        _ => 0,
    }
}

/// Snapshot of the IR input (works with a TSOP-style receiver or a 3-pin obstacle/proximity module).
#[derive(Clone)]
pub struct IrSnap {
    pub blocked: bool,
    pub falls: u32,
    pub edges: u32,
    pub rises: u32,
    pub last_edge_ms: u64,
    pub nec: Option<(u8, u8)>,
    pub nec_seq: u32,
    pub raw: [u16; 16],
    pub raw_n: usize,
    pub raw_seq: u32,
}

impl IrSnap {
    pub const fn new() -> Self {
        Self { blocked: false, falls: 0, edges: 0, rises: 0, last_edge_ms: 0, nec: None, nec_seq: 0, raw: [0; 16], raw_n: 0, raw_seq: 0 }
    }
}

impl Default for IrSnap {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Default)]
pub struct Info {
    pub heap_used: usize,
    pub heap_free: usize,
    pub fps: u32,
    pub ota_slot: &'static str,
    pub flash_mb: u32,
    pub storage_ok: bool,
    pub oled_a_ok: bool,
    pub oled_b_ok: bool,
}

pub struct Sys {
    pub now: u64,
    pub dt: u32,
    pub settings: Settings,
    pub wifi: WifiState,
    pub ir: IrSnap,
    pub info: Info,
    pub rng: u32,
    pub fx: Vec<Effect>,
    pub user: String,
    pub raw_keys: u16,
    pub tone_playing: bool,
    pub lock_req: bool,
}

impl Sys {
    pub fn new(settings: Settings, seed: u32) -> Self {
        Self {
            now: 0,
            dt: 0,
            settings,
            wifi: WifiState::default(),
            ir: IrSnap::default(),
            info: Info::default(),
            rng: seed | 1,
            fx: Vec::new(),
            user: String::new(),
            raw_keys: 0,
            tone_playing: false,
            lock_req: false,
        }
    }
    pub fn rand(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }
    pub fn rand_n(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.rand() % n }
    }
    pub fn snd(&mut self, s: Snd) {
        self.fx.push(Effect::Snd(s));
    }
    pub fn tone(&mut self, f: u16) {
        self.fx.push(Effect::Tone(f));
        self.tone_playing = true;
    }
    pub fn silence(&mut self) {
        self.fx.push(Effect::Silence);
        self.tone_playing = false;
    }
    pub fn play(&mut self, v: Vec<Step>) {
        self.fx.push(Effect::Play(v));
    }
    pub fn led_pulse(&mut self, ms: u16) {
        self.fx.push(Effect::LedPulse(ms));
    }
    pub fn save(&mut self) {
        self.fx.push(Effect::Save);
    }
    pub fn wifi_cmd(&mut self, c: WifiCmd) {
        self.fx.push(Effect::Wifi(c));
    }
    /// Seconds since the epoch in local time.
    pub fn local_epoch(&self) -> i64 {
        self.settings.epoch_off + (self.now / 1000) as i64 + self.settings.tz_min as i64 * 60
    }
    pub fn local_time(&self) -> crate::clock::DateTime {
        crate::clock::from_epoch(self.local_epoch())
    }
}
