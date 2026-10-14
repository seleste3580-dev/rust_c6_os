use aetheris::fb::Fb;
use aetheris::input::*;
use aetheris::os::Os;
use aetheris::settings::Settings;
use aetheris::sys::*;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    // script: "t:key,kind" tokens; e.g.  "3000 Hash Press" -- time in ms
    let script = std::fs::read_to_string(&args[1]).unwrap();
    let out_path = &args[2];
    let mut ev: Vec<(u64, Key, Kind)> = Vec::new();
    let mut snaps: Vec<u64> = Vec::new();
    for line in script.lines() {
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.is_empty() { continue; }
        let t: u64 = p[0].parse().unwrap();
        if p[1] == "snap" { snaps.push(t); continue; }
        let key = match p[1] {
            "0" => Key::K0, "1" => Key::K1, "2" => Key::K2, "3" => Key::K3, "4" => Key::K4,
            "5" => Key::K5, "6" => Key::K6, "7" => Key::K7, "8" => Key::K8, "9" => Key::K9,
            "A" => Key::A, "B" => Key::B, "C" => Key::C, "D" => Key::D, "*" => Key::Star, "#" => Key::Hash,
            _ => panic!("key"),
        };
        let kind = match p.get(2).copied().unwrap_or("p") { "l" => Kind::Long, "r" => Kind::Release, _ => Kind::Press };
        ev.push((t, key, kind));
    }
    let mut sys = Sys::new(Settings::default(), 0x1234_5678);
    sys.info = Info { heap_used: 61_000, heap_free: 120_000, fps: 20, ota_slot: "ota_0", flash_mb: 16, storage_ok: true, oled_a_ok: true, oled_b_ok: true, };
    sys.wifi.status = WifiStatus::Connected;
    sys.wifi.ssid = "SelesteNet".into();
    sys.wifi.rssi = -58;
    sys.wifi.ip = [192, 168, 1, 42];
    sys.wifi.aps = vec![
        ApRec { ssid: "SelesteNet".into(), rssi: -58, ch: 6, open: false },
        ApRec { ssid: "Cafe Guest".into(), rssi: -71, ch: 11, open: true },
        ApRec { ssid: "Safaricom_4G".into(), rssi: -80, ch: 1, open: false },
    ];
    let mut os = Os::new(&sys);
    let mut a = Fb::new();
    let mut b = Fb::new();
    let mut t = 1u64;
    let end = snaps.iter().max().copied().unwrap_or(0) + 100;
    let mut out = std::fs::File::create(out_path).unwrap();
    let mut ei = 0;
    while t <= end {
        sys.now = t;
        while ei < ev.len() && ev[ei].0 <= t {
            let e = KeyEvent { key: ev[ei].1, kind: ev[ei].2 };
            os.key(e, &mut sys);
            ei += 1;
        }
        os.tick(&mut sys);
        sys.fx.clear();
        if snaps.contains(&t) || snaps.iter().any(|&s| t >= s && t < s + 40 && t - s < 40 && (t % 40 == 0) && false) {
            os.draw(&mut a, &mut b, &sys);
            out.write_all(&a.buf).unwrap();
            out.write_all(&b.buf).unwrap();
        }
        t += 1;
        if t % 40 != 1 { /* step in 1ms to hit event times; tick frame cadence 40ms */ }
    }
}
