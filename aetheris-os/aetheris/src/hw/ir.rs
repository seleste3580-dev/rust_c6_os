//! IR input task: timestamps every edge on the sensor pin, decodes NEC remote frames and keeps
//! counters. Works with a TSOP-style receiver or a 3-pin obstacle/proximity module.
use super::shared::IR;
use crate::ir_decode::NecDecoder;
use embassy_time::Instant;
use esp_hal::gpio::{AnyPin, Input, InputConfig, Pull};

#[embassy_executor::task]
pub async fn ir_task(pin: AnyPin<'static>) {
    let mut input = Input::new(pin, InputConfig::default().with_pull(Pull::Up));
    let mut dec = NecDecoder::new();
    let mut last = Instant::now();
    IR.lock(|s| s.borrow_mut().blocked = input.is_low());
    loop {
        input.wait_for_any_edge().await;
        let now = Instant::now();
        let dur = (now - last).as_micros().min(u32::MAX as u64) as u32;
        if dur < 120 {
            continue; // glitch
        }
        last = now;
        let low_now = input.is_low();
        let was_low = !low_now;
        let code = dec.feed(was_low, dur);
        IR.lock(|s| {
            let mut s = s.borrow_mut();
            s.blocked = low_now;
            s.last_edge_ms = now.as_millis();
            if low_now {
                s.edges = s.edges.wrapping_add(1);
                if dur >= 30_000 {
                    s.falls = s.falls.wrapping_add(1);
                }
            } else {
                s.rises = s.rises.wrapping_add(1);
            }
            let n = s.raw_n.min(15);
            if s.raw_n >= 16 {
                s.raw.rotate_left(1);
                s.raw[15] = (dur / 100).min(u16::MAX as u32) as u16;
            } else {
                s.raw[n] = (dur / 100).min(u16::MAX as u32) as u16;
                s.raw_n = n + 1;
            }
            s.raw_seq = s.raw_seq.wrapping_add(1);
            if let Some(c) = code {
                s.nec = Some(c);
                s.nec_seq = s.nec_seq.wrapping_add(1);
            }
        });
    }
}
