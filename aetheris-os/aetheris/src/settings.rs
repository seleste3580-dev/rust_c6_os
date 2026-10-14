//! Persistent settings with a CRC-protected binary encoding (stored in the `storage` flash partition).
use alloc::string::String;
use alloc::vec::Vec;
use sha2::{Digest, Sha256};

pub const MAGIC: [u8; 4] = *b"SLS2";
pub const MAX_ENCODED: usize = 1024;

#[derive(Clone, PartialEq)]
pub struct Settings {
    pub user: String,
    pub salt: [u8; 8],
    pub hash: [u8; 32],
    pub wifi_ssid: String,
    pub wifi_pass: String,
    pub wp_a: u8,
    pub wp_b: u8,
    pub volume: u8,
    pub key_click: bool,
    pub boot_sound: bool,
    pub bright_a: u8,
    pub bright_b: u8,
    pub invert_a: bool,
    pub invert_b: bool,
    pub sleep_s: u16,
    pub lock_s: u16,
    pub tz_min: i16,
    pub epoch_off: i64,
    pub ir_wake: bool,
    pub wifi_auto: bool,
    pub note: String,
}

pub fn hash_pw(salt: &[u8; 8], pw: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(salt);
    h.update(pw.as_bytes());
    let out = h.finalize();
    let mut r = [0u8; 32];
    r.copy_from_slice(&out[..32]);
    r
}

impl Default for Settings {
    fn default() -> Self {
        let salt = *b"Seleste1";
        Self {
            user: String::from("admin"),
            salt,
            hash: hash_pw(&salt, "1234"),
            wifi_ssid: String::new(),
            wifi_pass: String::new(),
            wp_a: 0,
            wp_b: 1,
            volume: 4,
            key_click: true,
            boot_sound: true,
            bright_a: 3,
            bright_b: 3,
            invert_a: false,
            invert_b: false,
            sleep_s: 120,
            lock_s: 0,
            tz_min: 180,
            epoch_off: crate::clock::to_epoch(2026, 1, 1, 0, 0, 0),
            ir_wake: true,
            wifi_auto: true,
            note: String::new(),
        }
    }
}

impl Settings {
    pub fn check(&self, user: &str, pw: &str) -> bool {
        // Always hash so timing does not depend on the username.
        let h = hash_pw(&self.salt, pw);
        let mut diff = 0u8;
        for i in 0..32 {
            diff |= h[i] ^ self.hash[i];
        }
        user == self.user && diff == 0
    }
    pub fn set_password(&mut self, pw: &str, salt_seed: u32) {
        let mut salt = [0u8; 8];
        let mut x = salt_seed | 1;
        for b in salt.iter_mut() {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            *b = x as u8;
        }
        self.salt = salt;
        self.hash = hash_pw(&salt, pw);
    }
    pub fn is_default_login(&self) -> bool {
        self.check("admin", "1234")
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(512);
        v.extend_from_slice(&MAGIC);
        put_str(&mut v, &self.user);
        v.extend_from_slice(&self.salt);
        v.extend_from_slice(&self.hash);
        put_str(&mut v, &self.wifi_ssid);
        put_str(&mut v, &self.wifi_pass);
        v.extend_from_slice(&[self.wp_a, self.wp_b, self.volume, self.key_click as u8, self.boot_sound as u8]);
        v.extend_from_slice(&[self.bright_a, self.bright_b, self.invert_a as u8, self.invert_b as u8]);
        v.extend_from_slice(&self.sleep_s.to_le_bytes());
        v.extend_from_slice(&self.lock_s.to_le_bytes());
        v.extend_from_slice(&self.tz_min.to_le_bytes());
        v.extend_from_slice(&self.epoch_off.to_le_bytes());
        v.extend_from_slice(&[self.ir_wake as u8, self.wifi_auto as u8]);
        let n = self.note.as_bytes();
        let n = &n[..n.len().min(600)];
        v.extend_from_slice(&(n.len() as u16).to_le_bytes());
        v.extend_from_slice(n);
        let c = crc32(&v);
        v.extend_from_slice(&c.to_le_bytes());
        v
    }

    pub fn decode(b: &[u8]) -> Option<Settings> {
        if b.len() < 8 || b[..4] != MAGIC {
            return None;
        }
        let mut r = Rd { b, i: 4 };
        let mut s = Settings { user: r.str()?, ..Default::default() };
        s.salt.copy_from_slice(r.take(8)?);
        s.hash.copy_from_slice(r.take(32)?);
        s.wifi_ssid = r.str()?;
        s.wifi_pass = r.str()?;
        let f = r.take(5)?;
        s.wp_a = f[0];
        s.wp_b = f[1];
        s.volume = f[2].min(5);
        s.key_click = f[3] != 0;
        s.boot_sound = f[4] != 0;
        let f = r.take(4)?;
        s.bright_a = f[0].min(4);
        s.bright_b = f[1].min(4);
        s.invert_a = f[2] != 0;
        s.invert_b = f[3] != 0;
        s.sleep_s = u16::from_le_bytes(r.take(2)?.try_into().ok()?);
        s.lock_s = u16::from_le_bytes(r.take(2)?.try_into().ok()?);
        s.tz_min = i16::from_le_bytes(r.take(2)?.try_into().ok()?);
        s.epoch_off = i64::from_le_bytes(r.take(8)?.try_into().ok()?);
        let f = r.take(2)?;
        s.ir_wake = f[0] != 0;
        s.wifi_auto = f[1] != 0;
        let n = u16::from_le_bytes(r.take(2)?.try_into().ok()?) as usize;
        s.note = String::from_utf8(r.take(n)?.to_vec()).ok()?;
        let end = r.i;
        let crc = u32::from_le_bytes(r.take(4)?.try_into().ok()?);
        if crc32(&b[..end]) != crc {
            return None;
        }
        Some(s)
    }
}

fn put_str(v: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    let n = b.len().min(255);
    v.push(n as u8);
    v.extend_from_slice(&b[..n]);
}

struct Rd<'a> {
    b: &'a [u8],
    i: usize,
}
impl<'a> Rd<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.b.get(self.i..self.i + n)?;
        self.i += n;
        Some(s)
    }
    fn str(&mut self) -> Option<String> {
        let n = self.take(1)?[0] as usize;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = 0xFFFF_FFFFu32;
    for &b in data {
        c ^= b as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xEDB8_8320 } else { c >> 1 };
        }
    }
    !c
}
