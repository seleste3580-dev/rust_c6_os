//! 24x24 vector app icons drawn procedurally (no bitmap data needed).
use crate::fb::Fb;
use crate::gfx::{self, F};
use crate::trig::*;

pub const SIZE: i32 = 24;

fn arc(fb: &mut Fb, cx: i32, cy: i32, r: i32, a0: i32, a1: i32) {
    for i in a0..=a1 {
        fb.set(cx + cos64(i) * r / 127, cy + sin64(i) * r / 127, true);
    }
}

pub fn draw(fb: &mut Fb, icon: u8, x: i32, y: i32) {
    macro_rules! l {
        ($a:expr, $b:expr, $c:expr, $d:expr) => {
            fb.line(x + $a, y + $b, x + $c, y + $d, true)
        };
    }
    match icon {
        0 => {
            // gear
            for k in 0..8 {
                let (cx, cy) = (12 + cos64(k * 8) * 10 / 127, 12 + sin64(k * 8) * 10 / 127);
                fb.fill_rect(x + cx - 1, y + cy - 1, 3, 3, true);
            }
            fb.circle(x + 12, y + 12, 8, true);
            fb.circle(x + 12, y + 12, 7, true);
            fb.circle(x + 12, y + 12, 3, true);
        }
        1 => {
            for r in [5, 9, 13] {
                arc(fb, x + 12, y + 19, r, 38, 58);
            }
            fb.disc(x + 12, y + 19, 1, true);
        }
        2 => {
            fb.rect(x + 2, y + 4, 20, 16, true);
            for k in [6, 10, 14, 18] {
                fb.vline(x + k, y + 13, 7, true);
            }
            for k in [5, 9, 15, 19] {
                fb.fill_rect(x + k, y + 4, 3, 9, true);
            }
        }
        3 => {
            fb.disc(x + 6, y + 18, 3, true);
            fb.disc(x + 17, y + 16, 3, true);
            fb.vline(x + 8, y + 5, 14, true);
            fb.vline(x + 19, y + 3, 14, true);
            l!(8, 5, 19, 3);
            l!(8, 6, 19, 4);
            l!(8, 7, 19, 5);
        }
        4 => {
            fb.rect(x + 4, y + 2, 16, 20, true);
            fb.rect(x + 6, y + 4, 12, 5, true);
            for r in 0..3 {
                for c in 0..3 {
                    fb.fill_rect(x + 7 + c * 4, y + 12 + r * 3, 2, 2, true);
                }
            }
        }
        5 => {
            fb.circle(x + 12, y + 12, 10, true);
            l!(12, 12, 12, 5);
            l!(12, 12, 17, 15);
            for k in 0..4 {
                fb.set(x + 12 + cos64(k * 16) * 8 / 127, y + 12 + sin64(k * 16) * 8 / 127, true);
            }
        }
        6 => {
            fb.circle(x + 12, y + 14, 8, true);
            fb.rect(x + 10, y + 2, 4, 3, true);
            l!(12, 14, 12, 9);
            l!(18, 7, 20, 5);
            l!(12, 14, 15, 16);
        }
        7 => {
            l!(5, 3, 18, 3);
            l!(5, 21, 18, 21);
            l!(6, 4, 12, 12);
            l!(17, 4, 12, 12);
            l!(6, 20, 12, 12);
            l!(17, 20, 12, 12);
            fb.fill_rect(x + 8, y + 17, 7, 3, true);
            fb.vline(x + 12, y + 12, 5, true);
        }
        8 => {
            fb.rect(x + 5, y + 2, 14, 20, true);
            for k in 0..3 {
                l!(8, 7 + k * 4, 16, 7 + k * 4);
            }
            l!(8, 19, 12, 19);
        }
        9 => {
            for (a, b, c, d) in [(4, 18, 10, 18), (10, 18, 10, 12), (10, 12, 16, 12), (16, 12, 16, 6), (16, 6, 20, 6)] {
                l!(a, b, c, d);
                l!(a, b + 1, c, d + 1);
                l!(a + 1, b, c + 1, d);
            }
            fb.set(x + 19, y + 5, false);
            fb.fill_rect(x + 3, y + 5, 3, 3, true);
        }
        10 => {
            fb.fill_rect(x + 3, y + 5, 3, 9, true);
            fb.fill_rect(x + 18, y + 10, 3, 9, true);
            fb.disc(x + 12, y + 12, 2, true);
            for k in 0..4 {
                fb.vline(x + 12, y + 1 + k * 6, 3, true);
            }
        }
        11 => {
            fb.fill_rect(x + 3, y + 3, 8, 8, true);
            fb.rect(x + 13, y + 3, 8, 8, true);
            fb.rect(x + 3, y + 13, 8, 8, true);
            fb.fill_rect(x + 13, y + 13, 8, 8, true);
        }
        12 => {
            for (a, b, c, d) in [(14, 2, 6, 13), (6, 13, 11, 13), (11, 13, 9, 22), (9, 22, 18, 9), (18, 9, 12, 9), (12, 9, 14, 2)] {
                l!(a, b, c, d);
            }
            l!(13, 6, 11, 11);
            l!(12, 6, 10, 11);
        }
        13 => {
            gfx::frame(fb, x + 3, y + 3, 18, 18);
            for (a, b) in [(8, 8), (16, 8), (12, 12), (8, 16), (16, 16)] {
                fb.disc(x + a, y + b, 1, true);
            }
        }
        14 => {
            l!(7, 22, 10, 3);
            l!(17, 22, 14, 3);
            l!(10, 3, 14, 3);
            l!(7, 22, 17, 22);
            l!(12, 19, 17, 6);
            l!(13, 19, 18, 6);
            fb.fill_rect(x + 14, y + 9, 4, 3, true);
        }
        15 => {
            l!(2, 12, 22, 12);
            for xx in 2..22 {
                fb.set(x + xx, y + 12 + sin64(xx * 4) * 8 / 127, true);
                fb.set(x + xx, y + 13 + sin64(xx * 4) * 8 / 127, true);
            }
        }
        16 => {
            fb.disc(x + 5, y + 8, 2, true);
            fb.fill_rect(x + 9, y + 6, 7, 4, true);
            fb.disc(x + 20, y + 8, 2, true);
            fb.fill_rect(x + 3, y + 15, 7, 4, true);
            fb.disc(x + 14, y + 17, 2, true);
            fb.disc(x + 19, y + 17, 2, true);
        }
        17 => {
            fb.rect(x + 2, y + 8, 8, 8, true);
            fb.fill_rect(x + 4, y + 10, 4, 4, true);
            l!(11, 12, 20, 5);
            l!(11, 12, 22, 12);
            l!(11, 12, 20, 19);
            arc(fb, x + 11, y + 12, 7, 56, 72);
        }
        18 => {
            gfx::frame(fb, x + 2, y + 5, 20, 14);
            gfx::text(fb, x + 5, y + 8, "123", F::M6x10, true);
        }
        19 => {
            arc(fb, x + 12, y + 18, 11, 32, 64);
            l!(1, 18, 4, 18);
            l!(20, 18, 23, 18);
            l!(12, 18, 17, 10);
            l!(12, 19, 17, 11);
            fb.disc(x + 12, y + 18, 2, true);
        }
        20 => {
            for (i, (cx, cy, ch)) in [(2, 1, "0"), (2, 9, "1"), (2, 17, "A"), (9, 5, "7"), (9, 13, "Z"), (16, 1, "#"), (16, 9, "3"), (16, 17, "8")].iter().enumerate() {
                gfx::text(fb, x + cx + 1, y + cy, ch, F::S5x8, true);
                if i == 4 {
                    fb.invert_rect(x + cx, y + cy, 6, 8);
                }
            }
        }
        21 => {
            for (a, b) in [(10, 4), (14, 8), (6, 12), (10, 12), (14, 12)] {
                fb.fill_rect(x + a, y + b, 4, 4, true);
            }
            fb.rect(x + 1, y + 1, 22, 22, true);
        }
        22 => {
            for k in 0..4 {
                fb.fill_rect(x + 3 + k * 5, y + 18 - (k * 4 + 2), 3, k * 4 + 2, true);
            }
            fb.hline(x + 1, y + 20, 22, true);
        }
        23 => {
            fb.rect(x + 6, y + 6, 12, 12, true);
            fb.fill_rect(x + 10, y + 10, 4, 4, true);
            for k in [8, 11, 14, 17] {
                fb.vline(x + k - 1, y + 3, 3, true);
                fb.vline(x + k - 1, y + 18, 3, true);
                fb.hline(x + 3, y + k - 1, 3, true);
                fb.hline(x + 18, y + k - 1, 3, true);
            }
        }
        24 => {
            for r in 0..3 {
                for c in 0..3 {
                    if r == 1 && c == 1 {
                        fb.fill_rect(x + 3 + c * 7, y + 3 + r * 7, 5, 5, true);
                    } else {
                        fb.rect(x + 3 + c * 7, y + 3 + r * 7, 5, 5, true);
                    }
                }
            }
        }
        25 => {
            fb.disc(x + 12, y + 12, 4, true);
            for k in 0..8 {
                let (c, s) = (cos64(k * 8), sin64(k * 8));
                l!(12 + c * 7 / 127, 12 + s * 7 / 127, 12 + c * 11 / 127, 12 + s * 11 / 127);
            }
        }
        26 => {
            fb.circle(x + 8, y + 10, 4, true);
            fb.circle(x + 14, y + 8, 5, true);
            fb.circle(x + 18, y + 12, 3, true);
            fb.hline(x + 6, y + 15, 14, true);
            fb.fill_rect(x + 3, y + 8, 4, 7, false);
            fb.fill_rect(x + 20, y + 9, 3, 6, false);
            l!(12, 11, 12, 22);
            l!(12, 22, 8, 18);
            l!(12, 22, 16, 18);
        }
        27 => {
            arc(fb, x + 12, y + 11, 5, 32, 64);
            fb.vline(x + 7, y + 11, 3, true);
            fb.vline(x + 17, y + 11, 3, true);
            fb.fill_rect(x + 5, y + 13, 14, 9, true);
            fb.fill_rect(x + 11, y + 16, 2, 3, false);
        }
        28 => {
            // tomato (pomodoro)
            fb.disc(x + 12, y + 14, 9, true);
            fb.set(x + 8, y + 10, false);
            fb.set(x + 7, y + 11, false);
            fb.set(x + 7, y + 12, false);
            l!(12, 5, 12, 1);
            l!(12, 5, 8, 3);
            l!(12, 5, 16, 3);
        }
        29 => {
            // calendar page with binder rings
            fb.rect(x + 2, y + 4, 20, 18, true);
            fb.fill_rect(x + 2, y + 4, 20, 5, true);
            fb.fill_rect(x + 6, y + 2, 2, 4, true);
            fb.fill_rect(x + 16, y + 2, 2, 4, true);
            for r in 0..3 {
                for c in 0..4 {
                    fb.fill_rect(x + 5 + c * 4, y + 11 + r * 3, 2, 2, true);
                }
            }
        }
        30 => {
            // swap arrows
            l!(3, 8, 20, 8);
            l!(20, 8, 16, 4);
            l!(20, 8, 16, 12);
            l!(3, 16, 20, 16);
            l!(3, 16, 7, 12);
            l!(3, 16, 7, 20);
        }
        31 => {
            // playlist / queue bars
            for i in 0..4 {
                let y0 = 4 + i * 5;
                fb.fill_rect(x + 4, y + y0, 12, 3, true);
                fb.disc(x + 19, y + y0 + 1, 2, true);
            }
        }
        32 => {
            // proximity radar rings + alert
            fb.circle(x + 12, y + 14, 9, true);
            fb.circle(x + 12, y + 14, 5, true);
            fb.disc(x + 12, y + 14, 2, true);
            fb.fill_rect(x + 11, y + 2, 2, 5, true);
            fb.fill_rect(x + 11, y + 8, 2, 2, true);
        }
        33 => {
            // ninja runner silhouette
            fb.disc(x + 10, y + 6, 3, true);
            fb.fill_rect(x + 7, y + 10, 7, 6, true);
            l!(7, 16, 4, 21);
            l!(13, 16, 17, 21);
            l!(7, 12, 2, 15);
            fb.hline(x + 4, y + 5, 4, true);
            // cactus
            fb.fill_rect(x + 18, y + 10, 3, 12, true);
            fb.fill_rect(x + 16, y + 13, 2, 2, true);
        }
        _ => {
            fb.rect(x + 2, y + 2, 20, 20, true);
            gfx::text_c(fb, x + 12, y + 8, "?", F::M6x10, true);
        }
    }
}
