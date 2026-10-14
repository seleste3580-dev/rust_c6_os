//! The Aetheris shell: boot sequence, login, home screen, app grid, power management.
use crate::apps::widgets::{Res, TextPrompt};
use crate::apps::{self, App, Flow};
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::icons;
use crate::input::{Key, KeyEvent, Nav};
use crate::sound::Snd;
use crate::sys::{Sys, WifiStatus};
use crate::wallpaper::{Rain, Wall};
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub const FAVS: [usize; 4] = [0, 2, 4, 9];
const BOOT_BIOS_MS: u64 = 3600;
const BOOT_TOTAL_MS: u64 = 7000;
const MAX_FAILS: u8 = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Power {
    On,
    Dim,
    Off,
}

enum Screen {
    Boot,
    Login,
    Home,
    Menu,
    App(Box<dyn App>),
}

pub struct Os {
    screen: Screen,
    boot_t0: u64,
    boot_snd: bool,
    boot_lines_beeped: usize,
    rain: Rain,
    wall_a: Wall,
    wall_b: Wall,
    user: TextPrompt,
    pass: TextPrompt,
    stage: u8,
    fails: u8,
    locked_until: u64,
    err_until: u64,
    menu_sel: usize,
    from_menu: bool,
    last_input: u64,
    asleep: bool,
    last_status: WifiStatus,
    toast: Option<(String, u64)>,
    shake_until: u64,
    wake_ir_edges: u32,
}

impl Os {
    pub fn new(sys: &Sys) -> Self {
        let seed = sys.rng ^ 0xA5A5_1234;
        Self {
            screen: Screen::Boot,
            boot_t0: 0,
            boot_snd: false,
            boot_lines_beeped: 0,
            rain: Rain::new(seed),
            wall_a: Wall::new(sys.settings.wp_a, seed ^ 1),
            wall_b: Wall::new(sys.settings.wp_b, seed ^ 2),
            user: TextPrompt::new("Username", 15, false, ""),
            pass: TextPrompt::new("Password", 24, true, ""),
            stage: 0,
            fails: 0,
            locked_until: 0,
            err_until: 0,
            menu_sel: 0,
            from_menu: true,
            last_input: 0,
            asleep: false,
            last_status: WifiStatus::Off,
            toast: None,
            shake_until: 0,
            wake_ir_edges: sys.ir.falls,
        }
    }

    pub fn power(&self, sys: &Sys) -> Power {
        if self.asleep {
            return Power::Off;
        }
        let s = sys.settings.sleep_s as u64 * 1000;
        if s > 0 && !matches!(self.screen, Screen::Boot) && sys.now.saturating_sub(self.last_input) + 8000 >= s && !self.keep_awake() {
            return Power::Dim;
        }
        Power::On
    }

    fn keep_awake(&self) -> bool {
        match &self.screen {
            Screen::App(a) => a.keep_awake(),
            Screen::Boot => true,
            _ => false,
        }
    }

    pub fn sleep(&mut self) {
        self.asleep = true;
    }

    pub fn lock(&mut self, sys: &mut Sys) {
        if matches!(self.screen, Screen::Login | Screen::Boot) {
            return;
        }
        if let Screen::App(a) = &mut self.screen {
            a.on_exit(sys);
        }
        sys.silence();
        self.screen = Screen::Login;
        self.stage = 1;
        self.user = TextPrompt::new("Username", 15, false, &sys.settings.user.clone());
        self.pass = TextPrompt::new("Password", 24, true, "");
        sys.snd(Snd::Lock);
    }

    fn launch(&mut self, id: usize, from_menu: bool, sys: &mut Sys) {
        self.from_menu = from_menu;
        self.screen = Screen::App(apps::create(id));
        sys.snd(Snd::Select);
    }

    fn leave_app(&mut self, sys: &mut Sys) {
        if let Screen::App(a) = &mut self.screen {
            a.on_exit(sys);
        }
        sys.silence();
        self.screen = if self.from_menu { Screen::Menu } else { Screen::Home };
        sys.snd(Snd::Back);
    }

    pub fn key(&mut self, ev: KeyEvent, sys: &mut Sys) {
        let was_asleep = self.asleep;
        self.last_input = sys.now;
        self.asleep = false;
        if was_asleep {
            if ev.press() {
                self.wake(sys);
            }
            return;
        }
        if ev.press() && sys.settings.key_click && !matches!(self.screen, Screen::Login | Screen::App(_)) {
            sys.snd(Snd::Click);
        }
        // global: hold * to go home (apps that use * as a real key opt out)
        if ev.long() && ev.key == Key::Star {
            match &self.screen {
                Screen::App(a) if !a.owns_star() => {
                    self.from_menu = false;
                    self.leave_app(sys);
                    return;
                }
                Screen::Menu => {
                    self.screen = Screen::Home;
                    sys.snd(Snd::Back);
                    return;
                }
                _ => {}
            }
        }
        match &mut self.screen {
            Screen::Boot => {
                if ev.press() && sys.now - self.boot_t0 > 600 {
                    self.finish_boot(sys);
                }
            }
            Screen::Login => self.login_key(ev, sys),
            Screen::Home => {
                if !ev.press() {
                    return;
                }
                match ev.key {
                    Key::A | Key::K5 | Key::Hash => {
                        self.screen = Screen::Menu;
                        sys.snd(Snd::Select);
                    }
                    Key::B => self.launch(0, false, sys),
                    Key::C => self.lock(sys),
                    Key::D => self.launch(1, false, sys),
                    Key::K1 => self.launch(FAVS[0], false, sys),
                    Key::K2 => self.launch(FAVS[1], false, sys),
                    Key::K3 => self.launch(FAVS[2], false, sys),
                    Key::K4 => self.launch(FAVS[3], false, sys),
                    _ => {}
                }
            }
            Screen::Menu => {
                if !ev.down() {
                    return;
                }
                let n = apps::APPS.len();
                match ev.key.nav() {
                    Some(Nav::Left) => self.menu_sel = (self.menu_sel + n - 1) % n,
                    Some(Nav::Right) => self.menu_sel = (self.menu_sel + 1) % n,
                    Some(Nav::Up) => {
                        self.menu_sel = if self.menu_sel >= 4 { self.menu_sel - 4 } else { (self.menu_sel + n - 4 + (4 - n % 4) % 4) % n }
                    }
                    Some(Nav::Down) => self.menu_sel = if self.menu_sel + 4 < n { self.menu_sel + 4 } else { self.menu_sel % 4 },
                    Some(Nav::Ok) if ev.press() => {
                        let id = self.menu_sel;
                        self.launch(id, true, sys);
                    }
                    Some(Nav::Back) if ev.press() => {
                        self.screen = Screen::Home;
                        sys.snd(Snd::Back);
                    }
                    _ => {}
                }
            }
            Screen::App(a) => {
                let f = a.key(ev, sys);
                if f == Flow::Exit {
                    self.leave_app(sys);
                }
            }
        }
        if sys.lock_req {
            sys.lock_req = false;
            self.lock(sys);
        }
    }

    fn wake(&mut self, sys: &mut Sys) {
        let lock = sys.settings.lock_s;
        if lock > 0 && sys.now.saturating_sub(self.last_input) > 0 && !matches!(self.screen, Screen::Boot | Screen::Login) {
            // woke after a sleep: lock if auto-lock is on
            self.lock(sys);
        }
        sys.snd(Snd::Click);
    }

    fn finish_boot(&mut self, sys: &mut Sys) {
        sys.silence();
        self.screen = Screen::Login;
        self.stage = 0;
        sys.snd(Snd::Notify);
    }

    fn login_key(&mut self, ev: KeyEvent, sys: &mut Sys) {
        if sys.now < self.locked_until {
            return;
        }
        let stage = self.stage;
        let p = if stage == 0 { &mut self.user } else { &mut self.pass };
        match p.key(ev, sys) {
            Res::Submit => {
                if stage == 0 {
                    self.stage = 1;
                    sys.snd(Snd::Select);
                } else {
                    let (u, pw) = (String::from(self.user.text()), String::from(self.pass.text()));
                    if sys.settings.check(&u, &pw) {
                        sys.user = u;
                        self.fails = 0;
                        self.screen = Screen::Home;
                        sys.snd(Snd::Unlock);
                        self.pass = TextPrompt::new("Password", 24, true, "");
                        self.user = TextPrompt::new("Username", 15, false, "");
                        self.stage = 0;
                    } else {
                        self.fails += 1;
                        self.err_until = sys.now + 2000;
                        self.shake_until = sys.now + 400;
                        self.pass = TextPrompt::new("Password", 24, true, "");
                        sys.snd(Snd::Error);
                        if self.fails >= MAX_FAILS {
                            self.locked_until = sys.now + 15_000;
                            self.fails = 0;
                        }
                    }
                }
            }
            Res::Cancel => {
                // D toggles between the two fields
                self.stage = 1 - self.stage;
                sys.snd(Snd::Click);
            }
            Res::None => {}
        }
    }

    pub fn tick(&mut self, sys: &mut Sys) {
        if self.boot_t0 == 0 {
            self.boot_t0 = sys.now.max(1);
            self.last_input = sys.now;
        }
        // Wi-Fi status change notifications
        if sys.wifi.status != self.last_status {
            if sys.wifi.status == WifiStatus::Connected {
                sys.snd(Snd::Connect);
                self.toast = Some((format!("Wi-Fi: {}", gfx::clip(&sys.wifi.ssid, 14)), sys.now + 2500));
            } else if self.last_status == WifiStatus::Connected {
                sys.snd(Snd::Disconnect);
                self.toast = Some((String::from("Wi-Fi disconnected"), sys.now + 2500));
            } else if sys.wifi.status == WifiStatus::Failed {
                sys.snd(Snd::Error);
                self.toast = Some((String::from("Wi-Fi failed"), sys.now + 2500));
            }
            self.last_status = sys.wifi.status;
        }
        if let Some((_, t)) = &self.toast
            && sys.now > *t
        {
            self.toast = None;
        }
        // IR proximity wake
        if sys.ir.falls != self.wake_ir_edges {
            self.wake_ir_edges = sys.ir.falls;
            if sys.settings.ir_wake && self.asleep {
                self.asleep = false;
                self.last_input = sys.now;
                self.wake(sys);
            }
        }
        // idle handling
        let idle = sys.now.saturating_sub(self.last_input);
        let sleep_ms = sys.settings.sleep_s as u64 * 1000;
        let lock_ms = sys.settings.lock_s as u64 * 1000;
        if lock_ms > 0 && idle >= lock_ms && matches!(self.screen, Screen::Home | Screen::Menu | Screen::App(_)) && !self.keep_awake() {
            self.lock(sys);
        }
        if sleep_ms > 0 && idle >= sleep_ms && !self.keep_awake() {
            self.asleep = true;
        }
        match &mut self.screen {
            Screen::Boot => {
                let t = sys.now - self.boot_t0;
                self.rain.step(100);
                if t >= BOOT_BIOS_MS && !self.boot_snd {
                    self.boot_snd = true;
                    if sys.settings.boot_sound {
                        sys.snd(Snd::Boot);
                    }
                }
                let lines = ((t / 300) as usize).min(12);
                while self.boot_lines_beeped < lines && t < BOOT_BIOS_MS {
                    self.boot_lines_beeped += 1;
                    sys.snd(Snd::BootBeep);
                }
                if t >= BOOT_TOTAL_MS {
                    self.finish_boot(sys);
                }
            }
            Screen::Login => {
                self.rain.step(70);
                self.user.tick(sys);
                self.pass.tick(sys);
            }
            Screen::Home | Screen::Menu => {
                self.wall_a.set_kind(sys.settings.wp_a);
                self.wall_b.set_kind(sys.settings.wp_b);
                self.wall_a.step();
                self.wall_b.step();
            }
            Screen::App(a) => a.tick(sys),
        }
        if sys.lock_req {
            sys.lock_req = false;
            self.lock(sys);
        }
    }

    // ───────────────────────────── drawing ─────────────────────────────
    pub fn draw(&mut self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        a.clear();
        b.clear();
        match &self.screen {
            Screen::Boot => self.draw_boot(a, b, sys),
            Screen::Login => self.draw_login(a, b, sys),
            Screen::Home => self.draw_home(a, b, sys),
            Screen::Menu => self.draw_menu(a, b, sys),
            Screen::App(app) => {
                app.draw(a, b, sys);
                // status icons in the title band if there is room
                let free = (112..127).all(|x| (2..12).all(|y| !a.get(x, y)));
                if free && sys.wifi.connected() {
                    gfx::wifi_icon(a, 114, 4, sys.wifi.bars(), true);
                }
            }
        }
        if let Some((m, _)) = &self.toast {
            a.fill_rect(0, 51, 128, 13, true);
            gfx::text_c(a, 64, 53, m, F::M6x10, false);
        }
    }

    fn draw_boot(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        let t = sys.now.saturating_sub(self.boot_t0);
        self.rain.draw(b);
        if t < BOOT_BIOS_MS {
            let heap = (sys.info.heap_used + sys.info.heap_free) / 1024;
            let lines: [String; 12] = [
                String::from("SELESTE BIOS v1.0"),
                String::from("CPU  ESP32-C6 RV32 160MHz"),
                format!("RAM  {} KB ........ OK", heap),
                format!("FLASH {} MB ....... OK", sys.info.flash_mb),
                format!("OLED-A 128x64 ..... {}", if sys.info.oled_a_ok { "OK" } else { "--" }),
                format!("OLED-B 128x64 ..... {}", if sys.info.oled_b_ok { "OK" } else { "--" }),
                String::from("KEYPAD 4x4 ....... OK"),
                String::from("IR SENSOR ........ OK"),
                String::from("BUZZER ........... OK"),
                String::from("WI-FI 802.11ax .... OK"),
                format!("STORAGE .......... {}", if sys.info.storage_ok { "OK" } else { "NEW" }),
                String::from("Loading Aetheris..."),
            ];
            let n = ((t / 300) as usize + 1).min(12);
            let first = n.saturating_sub(7);
            for (i, l) in lines[first..n].iter().enumerate() {
                gfx::text(a, 1, 1 + i as i32 * 9, l, F::S5x8, true);
            }
            if (t / 200) % 2 == 0 {
                a.fill_rect(1, 1 + (n - first) as i32 * 9, 5, 7, true);
            }
            // keep a clear box on the rain for readability
            b.fill_rect(20, 24, 88, 16, false);
            gfx::frame(b, 20, 24, 88, 16);
            gfx::text_c(b, 64, 28, "BOOTING...", F::M6x10, true);
        } else {
            let p = (t - BOOT_BIOS_MS) as i32;
            let span = (BOOT_TOTAL_MS - BOOT_BIOS_MS) as i32;
            gfx::text_c(a, 64, 3, "SELESTE", F::H10x20, true);
            gfx::text_c(a, 64, 25, "TECHNOLOGIES", F::M6x10, true);
            a.hline(14, 37, 100, true);
            gfx::text_c(a, 64, 41, "AETHERIS OS", F::M6x10, true);
            gfx::bar(a, 14, 53, 100, 9, p, span);
            // logo reveal on the second screen: rain parts, name appears
            let reveal = (p * 110 / span).min(46);
            b.fill_rect(64 - reveal - 2, 18, reveal * 2 + 4, 28, false);
            if reveal >= 44 {
                gfx::frame(b, 64 - reveal - 2, 18, reveal * 2 + 4, 28);
                gfx::text_c(b, 64, 21, "SELESTE", F::B9x15, true);
                gfx::text_c(b, 64, 36, "TECHNOLOGIES", F::M6x10, true);
            }
        }
    }

    fn draw_login(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        let locked = sys.now < self.locked_until;
        let shake = if sys.now < self.shake_until { ((sys.now / 40) % 2) as i32 * 4 - 2 } else { 0 };
        gfx::title(a, "LOGIN");
        gfx::text_r(a, 125, 3, self.pass.entry.mode.label(), F::M6x10, true);
        let (uc, pc) = if self.stage == 0 { (true, false) } else { (false, true) };
        let field = |a: &mut Fb, y: i32, label: &str, txt: &str, active: bool| {
            gfx::text(a, 3 + shake, y + 3, label, F::M6x10, true);
            if active {
                gfx::frame(a, 33 + shake, y, 92, 14);
            } else {
                a.hline(33 + shake, y + 13, 92, true);
            }
            let n = txt.chars().count();
            let shown: String = txt.chars().skip(n.saturating_sub(14)).collect();
            gfx::text(a, 36 + shake, y + 2, &shown, F::M6x10, true);
            if active && (sys.now / 450) % 2 == 0 {
                a.vline(36 + shake + shown.chars().count() as i32 * 6, y + 2, 10, true);
            }
        };
        field(a, 18, "User", &self.user.shown(), uc);
        field(a, 35, "Pass", &self.pass.shown(), pc);
        if locked {
            gfx::text_c(a, 64, 53, &format!("LOCKED {}s", (self.locked_until - sys.now) / 1000 + 1), F::M6x10, true);
        } else if sys.now < self.err_until {
            gfx::text_c(a, 64, 53, &format!("Access denied {}/{}", self.fails.max(1), MAX_FAILS), F::M6x10, true);
        } else if sys.settings.is_default_login() {
            gfx::text_c(a, 64, 54, "default: admin / 1234", F::S5x8, true);
        } else {
            gfx::text_c(a, 64, 54, "#:next/ok  D:switch field", F::S5x8, true);
        }
        // second screen: faded rain + keypad legend
        if self.stage == 0 { self.user.legend(b) } else { self.pass.legend(b) }
        let _ = &self.rain;
    }

    fn draw_home(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        self.wall_a.draw(a);
        self.wall_b.draw(b);
        a.fill_rect(0, 0, 128, 16, false);
        gfx::text(a, 3, 1, "AETHERIS", F::B9x15, true);
        a.hline(0, 15, 128, true);
        gfx::wifi_icon(a, 104, 4, sys.wifi.bars(), sys.wifi.connected());
        gfx::speaker_icon(a, 88, 3, sys.settings.volume);
        let t = sys.local_time();
        a.fill_rect(12, 20, 104, 36, false);
        gfx::frame(a, 12, 20, 104, 36);
        gfx::text_c(a, 64, 23, &format!("{:02}:{:02}", t.h, t.mi), F::H10x20, true);
        gfx::text(a, 100, 38, &format!("{:02}", t.s), F::S5x8, true);
        let d = format!("{} {} {}", crate::clock::DAYS[t.wd as usize], t.d, crate::clock::MONTHS[(t.mo - 1) as usize]);
        gfx::text_c(a, 64, 45, &d, F::M6x10, true);
        // dock on the second screen: favourite apps with shortcut numbers
        b.fill_rect(0, 31, 128, 33, false);
        b.hline(0, 31, 128, true);
        for (i, &id) in FAVS.iter().enumerate() {
            let x = 6 + i as i32 * 31;
            icons::draw(b, apps::APPS[id].icon, x + 1, 33);
            gfx::text_c(b, x + 13, 58, &format!("{}", i + 1), F::S5x8, true);
        }
        // user chip
        if !sys.user.is_empty() {
            gfx::text_inv(b, 2, 2, &gfx::clip(&sys.user, 12), F::S5x8);
        }
        let _ = Key::A;
    }

    fn draw_menu(&self, a: &mut Fb, b: &mut Fb, sys: &Sys) {
        let n = apps::APPS.len();
        let page = self.menu_sel / 8;
        let pages = n.div_ceil(8);
        gfx::title(a, "APPS");
        gfx::text_r(a, 109, 3, &format!("{}/{}", page + 1, pages), F::M6x10, true);
        gfx::wifi_icon(a, 114, 4, sys.wifi.bars(), sys.wifi.connected());
        let info = &apps::APPS[self.menu_sel];
        gfx::text_c(a, 64, 19, &gfx::clip(info.name, 14), F::XL9x15, true);
        gfx::text_c(a, 64, 36, &gfx::clip(info.desc, 21), F::M6x10, true);
        for p in 0..pages {
            let x = 64 - pages as i32 * 4 + p as i32 * 8 + 1;
            if p == page {
                a.fill_rect(x, 49, 5, 3, true);
            } else {
                a.rect(x, 49, 5, 3, true);
            }
        }
        gfx::hint(a, "arrows move  #open  *home");
        for slot in 0..8 {
            let id = page * 8 + slot;
            if id >= n {
                break;
            }
            let (cx, cy) = ((slot % 4) as i32 * 32, (slot / 4) as i32 * 32);
            icons::draw(b, apps::APPS[id].icon, cx + 4, cy + 4);
            if id == self.menu_sel {
                b.invert_rect(cx + 1, cy + 1, 30, 30);
            }
        }
    }
}

pub fn _unused(_: Vec<u8>) {}
