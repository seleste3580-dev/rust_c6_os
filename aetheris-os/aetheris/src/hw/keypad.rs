//! 4x4 matrix keypad scanner. Rows are open-drain outputs (one pulled low at a time), columns are
//! inputs with pull-ups. Returns a 16-bit mask: bit (row*4 + col).
use esp_hal::delay::Delay;
use esp_hal::gpio::{AnyPin, DriveMode, Input, InputConfig, Level, Output, OutputConfig, Pull};

pub struct Keypad<'d> {
    rows: [Output<'d>; 4],
    cols: [Input<'d>; 4],
    delay: Delay,
}

impl<'d> Keypad<'d> {
    pub fn new(rows: [AnyPin<'d>; 4], cols: [AnyPin<'d>; 4]) -> Self {
        let oc = OutputConfig::default().with_drive_mode(DriveMode::OpenDrain);
        let ic = InputConfig::default().with_pull(Pull::Up);
        let [r0, r1, r2, r3] = rows;
        let [c0, c1, c2, c3] = cols;
        Self {
            rows: [
                Output::new(r0, Level::High, oc),
                Output::new(r1, Level::High, oc),
                Output::new(r2, Level::High, oc),
                Output::new(r3, Level::High, oc),
            ],
            cols: [Input::new(c0, ic), Input::new(c1, ic), Input::new(c2, ic), Input::new(c3, ic)],
            delay: Delay::new(),
        }
    }
    pub fn scan(&mut self) -> u16 {
        let mut mask = 0u16;
        for r in 0..4 {
            self.rows[r].set_low();
            self.delay.delay_micros(25);
            for c in 0..4 {
                if self.cols[c].is_low() {
                    mask |= 1 << (r * 4 + c);
                }
            }
            self.rows[r].set_high();
        }
        mask
    }
}
