use aetheris::input::*;
use aetheris::settings::*;
use aetheris::notes;
use aetheris::clock;

#[test]
fn multitap() {
    let mut t = TextEntry::new(10);
    // "hi": 4,4 -> h ; (different key 4 same, so 4 4 = h,i)  type h: 4 once; i: 4 twice
    t.key(Key::K4, 0); t.key(Key::K4, 200); t.tick(2000);
    t.key(Key::K4, 3000); t.key(Key::K4, 3300); t.key(Key::K4, 3500);
    assert_eq!(t.text, "hi");
    t.key(Key::K0, 5000);
    assert_eq!(t.text, "hi ");
    t.key(Key::K0, 5300);
    assert_eq!(t.text, "hi0");
    t.cycle_mode(); // Upper
    t.key(Key::K2, 9000);
    assert_eq!(t.text, "hi0A");
    t.key(Key::K2, 9200);
    assert_eq!(t.text, "hi0B");
    t.cycle_mode(); // digits
    t.key(Key::K7, 9300);
    assert_eq!(t.text, "hi0B7");
    t.backspace();
    assert_eq!(t.text, "hi0B");
}

#[test]
fn freqs() {
    assert_eq!(notes::freq(69), 440);
    assert_eq!(notes::freq(60), 262);
    assert_eq!(notes::freq(72), 523);
    assert_eq!(notes::freq(81), 880);
    assert_eq!(notes::freq(61), 277);
    assert!((notes::freq_x100(60) as i32 - 26163).abs() < 2);
    assert_eq!(notes::name(61), "C#");
    assert_eq!(notes::octave(60), 4);
}

#[test]
fn settings_roundtrip() {
    let mut s = Settings::default();
    s.user = "waithaka".into();
    s.wifi_ssid = "Net".into();
    s.wifi_pass = "pw12345678".into();
    s.note = "hello note".into();
    s.set_password("s3cret!", 1234);
    let e = s.encode();
    assert!(e.len() < MAX_ENCODED);
    let d = Settings::decode(&e).unwrap();
    assert!(d == s);
    assert!(d.check("waithaka", "s3cret!"));
    assert!(!d.check("waithaka", "1234"));
    let mut bad = e.clone();
    bad[10] ^= 1;
    assert!(Settings::decode(&bad).is_none());
    assert!(Settings::default().is_default_login());
}

#[test]
fn dates() {
    let e = clock::to_epoch(2026, 10, 14, 12, 30, 15);
    let d = clock::from_epoch(e);
    assert_eq!((d.y, d.mo, d.d, d.h, d.mi, d.s), (2026, 10, 14, 12, 30, 15));
    assert_eq!(clock::DAYS[d.wd as usize], "Wed");
    assert_eq!(clock::from_epoch(0).y, 1970);
}

#[test]
fn scanner() {
    let mut sc = KeyScanner::new();
    let mut out = vec![];
    // press key idx 0 (row0,col0 = '1')
    for t in 0..3 { sc.update(1, t * 10, &mut out); }
    assert_eq!(out.len(), 1);
    assert_eq!(out[0], KeyEvent { key: Key::K1, kind: Kind::Press });
    out.clear();
    for t in 3..100 { sc.update(1, t * 10, &mut out); }
    assert!(out.iter().any(|e| e.kind == Kind::Long));
    out.clear();
    for t in 100..103 { sc.update(0, t * 10, &mut out); }
    assert_eq!(out[0].kind, Kind::Release);
    // bounce: single-sample glitch ignored
    out.clear();
    sc.update(2, 2000, &mut out); sc.update(0, 2010, &mut out); sc.update(0, 2020, &mut out);
    assert!(out.is_empty());
}

#[test]
fn melody_len() {
    for t in aetheris::tunes::TUNES.iter() {
        let st = aetheris::sound::melody(t.notes, t.bpm);
        let total: u32 = st.iter().map(|s| s.ms as u32).sum();
        assert!(total > 3000 && total < 40000, "{} {}", t.name, total);
    }
}

#[test]
fn nec() {
    use aetheris::ir_decode::NecDecoder;
    let mut d = NecDecoder::new();
    let (addr, cmd) = (0x00u8, 0x45u8);
    let word: u32 = addr as u32 | ((!addr as u32) << 8) | ((cmd as u32) << 16) | ((!cmd as u32) << 24);
    assert_eq!(d.feed(true, 9000), None);
    assert_eq!(d.feed(false, 4500), None);
    let mut res = None;
    for i in 0..32 {
        assert_eq!(res, None);
        d.feed(true, 560);
        res = d.feed(false, if word >> i & 1 == 1 { 1690 } else { 560 });
    }
    assert_eq!(res, Some((0x00, 0x45)));
}
