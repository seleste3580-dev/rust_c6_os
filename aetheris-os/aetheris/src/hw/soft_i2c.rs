//! Bit-banged I2C master (write-oriented) so the second OLED can sit on its own pins
//! and both panels may keep the same 0x3C address.
use embedded_hal::i2c::{Error, ErrorKind, ErrorType, I2c, NoAcknowledgeSource, Operation};
use esp_hal::delay::Delay;
use esp_hal::gpio::{DriveMode, Flex, InputConfig, OutputConfig, Pull};

pub struct SoftI2c<'d> {
    sda: Flex<'d>,
    scl: Flex<'d>,
    delay: Delay,
    half_us: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct SoftError(ErrorKind);
impl Error for SoftError {
    fn kind(&self) -> ErrorKind {
        self.0
    }
}

impl<'d> SoftI2c<'d> {
    pub fn new(sda: impl esp_hal::gpio::Pin + 'd, scl: impl esp_hal::gpio::Pin + 'd, half_us: u32) -> Self {
        let mut sda = Flex::new(sda);
        let mut scl = Flex::new(scl);
        let oc = OutputConfig::default().with_drive_mode(DriveMode::OpenDrain).with_pull(Pull::Up);
        for p in [&mut sda, &mut scl] {
            p.apply_output_config(&oc);
            p.apply_input_config(&InputConfig::default().with_pull(Pull::Up));
            p.set_high();
            p.set_output_enable(true);
            p.set_input_enable(true);
        }
        Self { sda, scl, delay: Delay::new(), half_us }
    }
    #[inline]
    fn d(&self) {
        self.delay.delay_micros(self.half_us);
    }
    fn start(&mut self) {
        self.sda.set_high();
        self.scl.set_high();
        self.d();
        self.sda.set_low();
        self.d();
        self.scl.set_low();
    }
    fn stop(&mut self) {
        self.sda.set_low();
        self.d();
        self.scl.set_high();
        self.d();
        self.sda.set_high();
        self.d();
    }
    fn write_byte(&mut self, b: u8) -> bool {
        for i in (0..8).rev() {
            if b >> i & 1 == 1 {
                self.sda.set_high();
            } else {
                self.sda.set_low();
            }
            self.d();
            self.scl.set_high();
            self.d();
            self.scl.set_low();
        }
        self.sda.set_high();
        self.d();
        self.scl.set_high();
        self.d();
        let ack = self.sda.is_low();
        self.scl.set_low();
        ack
    }
    fn read_byte(&mut self, ack: bool) -> u8 {
        let mut v = 0;
        self.sda.set_high();
        for _ in 0..8 {
            self.d();
            self.scl.set_high();
            self.d();
            v = v << 1 | self.sda.is_high() as u8;
            self.scl.set_low();
        }
        if ack {
            self.sda.set_low();
        } else {
            self.sda.set_high();
        }
        self.d();
        self.scl.set_high();
        self.d();
        self.scl.set_low();
        self.sda.set_high();
        v
    }
}

impl ErrorType for SoftI2c<'_> {
    type Error = SoftError;
}

impl I2c for SoftI2c<'_> {
    fn transaction(&mut self, address: u8, operations: &mut [Operation<'_>]) -> Result<(), Self::Error> {
        let mut res = Ok(());
        for op in operations.iter_mut() {
            self.start();
            match op {
                Operation::Write(buf) => {
                    if !self.write_byte(address << 1) {
                        res = Err(SoftError(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)));
                        break;
                    }
                    for &b in buf.iter() {
                        if !self.write_byte(b) {
                            res = Err(SoftError(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Data)));
                            break;
                        }
                    }
                }
                Operation::Read(buf) => {
                    if !self.write_byte(address << 1 | 1) {
                        res = Err(SoftError(ErrorKind::NoAcknowledge(NoAcknowledgeSource::Address)));
                        break;
                    }
                    let n = buf.len();
                    for (i, b) in buf.iter_mut().enumerate() {
                        *b = self.read_byte(i + 1 < n);
                    }
                }
            }
            if res.is_err() {
                break;
            }
        }
        self.stop();
        res
    }
}
