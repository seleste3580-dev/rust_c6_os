//! NEC infrared frame decoder fed with (level, duration) pulses, plus helpers for raw capture.

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    Idle,
    LeaderHigh,
    BitLow,
    BitHigh,
}

pub struct NecDecoder {
    stage: Stage,
    bits: u32,
    n: u8,
}

impl Default for NecDecoder {
    fn default() -> Self {
        Self::new()
    }
}

fn within(v: u32, lo: u32, hi: u32) -> bool {
    v >= lo && v <= hi
}

impl NecDecoder {
    pub const fn new() -> Self {
        Self { stage: Stage::Idle, bits: 0, n: 0 }
    }
    /// Feed one finished pulse: `was_low` = the (active-low) receiver output was low for `us` microseconds.
    /// Returns (address, command) when a complete, valid frame has been received.
    pub fn feed(&mut self, was_low: bool, us: u32) -> Option<(u8, u8)> {
        match self.stage {
            Stage::Idle => {
                if was_low && within(us, 7500, 10500) {
                    self.stage = Stage::LeaderHigh;
                }
            }
            Stage::LeaderHigh => {
                if !was_low && within(us, 3500, 5500) {
                    self.stage = Stage::BitLow;
                    self.bits = 0;
                    self.n = 0;
                } else {
                    self.stage = Stage::Idle;
                }
            }
            Stage::BitLow => {
                if was_low && within(us, 300, 900) {
                    self.stage = Stage::BitHigh;
                } else {
                    self.stage = Stage::Idle;
                    return self.feed(was_low, us);
                }
            }
            Stage::BitHigh => {
                if was_low {
                    self.stage = Stage::Idle;
                    return self.feed(was_low, us);
                }
                let bit = if within(us, 300, 900) {
                    0
                } else if within(us, 1300, 2100) {
                    1
                } else {
                    self.stage = Stage::Idle;
                    return None;
                };
                self.bits |= bit << self.n;
                self.n += 1;
                if self.n == 32 {
                    self.stage = Stage::Idle;
                    let b = self.bits;
                    let (addr, cmd, ncmd) = (b as u8, (b >> 16) as u8, (b >> 24) as u8);
                    if cmd == !ncmd {
                        return Some((addr, cmd));
                    }
                } else {
                    self.stage = Stage::BitLow;
                }
            }
        }
        None
    }
}
