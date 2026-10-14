//! Wi-Fi station manager (scan / connect / auto-reconnect / DHCP), SNTP time sync and the
//! wireless firmware-update (OTA) HTTP endpoint.
use super::shared::{OTA_ON, TIME_SYNC, WIFI_CMD, WIFI_STATE};
use super::storage::FLASH;
use crate::sys::{ApRec, OtaState, WifiCmd, WifiState, WifiStatus};
use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use embassy_futures::select::{Either, Either3, select, select3};
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{Runner, Stack};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use embedded_io_async::Write;
use esp_bootloader_esp_idf::ota::OtaImageState;
use esp_bootloader_esp_idf::partitions::{AppPartitionSubType, DataPartitionSubType, PARTITION_TABLE_MAX_LEN, PartitionType, read_partition_table};
use esp_radio::wifi::scan::ScanConfig;
use esp_radio::wifi::sta::StationConfig;
use esp_radio::wifi::{AuthenticationMethod, AuthenticationMethodConfig, Config, Interface, Ssid, WifiController};

fn upd(f: impl FnOnce(&mut WifiState)) {
    WIFI_STATE.lock(|s| f(&mut s.borrow_mut()));
}

#[embassy_executor::task]
pub async fn net_runner(mut runner: Runner<'static, Interface>) {
    runner.run().await
}

struct Mgr {
    ctrl: WifiController<'static>,
    stack: Stack<'static>,
    want: Option<(String, String)>,
    enabled: bool,
    connected: bool,
    retry_at: Instant,
}

impl Mgr {
    fn idle_status(&self) -> WifiStatus {
        if !self.enabled {
            WifiStatus::Off
        } else if self.connected {
            WifiStatus::Connected
        } else {
            WifiStatus::Idle
        }
    }

    async fn connect(&mut self, ssid: &str, pass: &str) {
        upd(|s| {
            s.status = WifiStatus::Connecting;
            s.ssid = String::from(ssid);
            s.err = "";
        });
        let auth = if pass.is_empty() {
            AuthenticationMethodConfig::Open
        } else {
            match pass.try_into() {
                Ok(p) => AuthenticationMethodConfig::Wpa2Personal(p),
                Err(_) => {
                    upd(|s| {
                        s.status = WifiStatus::Failed;
                        s.err = "bad password length";
                    });
                    return;
                }
            }
        };
        let Ok(ssid_t) = Ssid::try_from(ssid) else {
            upd(|s| {
                s.status = WifiStatus::Failed;
                s.err = "bad ssid";
            });
            return;
        };
        let cfg = Config::Station(StationConfig::default().with_ssid(ssid_t).with_authentication(auth));
        if self.ctrl.set_config(&cfg).is_err() {
            upd(|s| {
                s.status = WifiStatus::Failed;
                s.err = "config error";
            });
            return;
        }
        match with_timeout(Duration::from_secs(15), self.ctrl.connect_async()).await {
            Ok(Ok(_)) => {}
            Ok(Err(_)) => {
                upd(|s| {
                    s.status = WifiStatus::Failed;
                    s.err = "auth/connect failed";
                });
                self.retry_at = Instant::now() + Duration::from_secs(12);
                return;
            }
            Err(_) => {
                upd(|s| {
                    s.status = WifiStatus::Failed;
                    s.err = "timeout";
                });
                self.retry_at = Instant::now() + Duration::from_secs(12);
                return;
            }
        }
        if with_timeout(Duration::from_secs(12), self.stack.wait_config_up()).await.is_err() {
            upd(|s| {
                s.status = WifiStatus::Failed;
                s.err = "no IP (DHCP)";
            });
            return;
        }
        self.connected = true;
        self.refresh();
        // sync the clock once per connection
        if let Some(t) = sntp(self.stack).await {
            TIME_SYNC.signal(t);
            upd(|s| s.time_synced = true);
        }
    }

    fn refresh(&mut self) {
        let ip = self.stack.config_v4().map(|c| c.address.address().octets()).unwrap_or([0; 4]);
        let rssi = self.ctrl.rssi().unwrap_or(-100) as i8;
        let connected = self.connected;
        upd(|s| {
            s.ip = ip;
            s.rssi = rssi;
            if connected {
                s.status = WifiStatus::Connected;
            }
        });
    }

    async fn scan(&mut self) {
        if !self.enabled {
            return;
        }
        upd(|s| s.status = WifiStatus::Scanning);
        let res = self.ctrl.scan_async(&ScanConfig::default().with_max(30)).await;
        let mut aps: Vec<ApRec> = Vec::new();
        if let Ok(list) = res {
            for ap in list {
                let ssid = String::from(ap.ssid.as_str());
                if ssid.is_empty() || aps.iter().any(|a| a.ssid == ssid) {
                    continue;
                }
                aps.push(ApRec {
                    ssid,
                    rssi: ap.signal_strength,
                    ch: ap.channel,
                    open: matches!(ap.auth_method, None | Some(AuthenticationMethod::None)),
                });
            }
        }
        aps.sort_by(|a, b| b.rssi.cmp(&a.rssi));
        aps.truncate(20);
        let st = self.idle_status();
        upd(|s| {
            s.aps = aps;
            s.scan_seq = s.scan_seq.wrapping_add(1);
            s.status = st;
        });
    }

    async fn handle(&mut self, cmd: WifiCmd) {
        match cmd {
            WifiCmd::Scan => self.scan().await,
            WifiCmd::Connect { ssid, pass } => {
                if self.connected {
                    let _ = self.ctrl.disconnect_async().await;
                    self.connected = false;
                }
                self.enabled = true;
                self.want = Some((ssid, pass));
                self.retry_at = Instant::now();
            }
            WifiCmd::Disconnect => {
                self.want = None;
                if self.connected {
                    let _ = self.ctrl.disconnect_async().await;
                    self.connected = false;
                }
                let st = self.idle_status();
                upd(|s| s.status = st);
            }
            WifiCmd::SetEnabled(on) => {
                self.enabled = on;
                if !on && self.connected {
                    let _ = self.ctrl.disconnect_async().await;
                    self.connected = false;
                }
                if on {
                    self.retry_at = Instant::now();
                }
                let st = self.idle_status();
                upd(|s| s.status = st);
            }
            WifiCmd::OtaServer(on) => {
                OTA_ON.store(on && self.connected, Ordering::Relaxed);
                upd(|s| s.ota = if on && self.connected { OtaState::Listening } else { OtaState::Idle });
            }
            WifiCmd::SyncTime => {
                if self.connected {
                    if let Some(t) = sntp(self.stack).await {
                        TIME_SYNC.signal(t);
                        upd(|s| s.time_synced = true);
                    }
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn wifi_task(ctrl: WifiController<'static>, stack: Stack<'static>, mac: [u8; 6], saved: Option<(String, String)>, auto: bool) {
    let mut m = Mgr { ctrl, stack, want: if auto { saved } else { None }, enabled: true, connected: false, retry_at: Instant::now() };
    let _ = m.ctrl.set_config(&Config::Station(StationConfig::default()));
    upd(|s| {
        s.mac = mac;
        s.status = WifiStatus::Idle;
    });
    loop {
        if m.enabled && !m.connected && Instant::now() >= m.retry_at {
            if let Some((ssid, pass)) = m.want.clone() {
                m.connect(&ssid, &pass).await;
                if !m.connected {
                    m.retry_at = Instant::now() + Duration::from_secs(12);
                }
            }
        }
        if m.connected {
            match select3(WIFI_CMD.receive(), m.ctrl.wait_for_disconnect_async(), Timer::after(Duration::from_secs(2))).await {
                Either3::First(c) => m.handle(c).await,
                Either3::Second(_) => {
                    m.connected = false;
                    OTA_ON.store(false, Ordering::Relaxed);
                    upd(|s| {
                        s.status = WifiStatus::Idle;
                        s.ota = OtaState::Idle;
                        s.ip = [0; 4];
                    });
                    m.retry_at = Instant::now() + Duration::from_secs(3);
                }
                Either3::Third(_) => m.refresh(),
            }
        } else {
            match select(WIFI_CMD.receive(), Timer::after(Duration::from_secs(2))).await {
                Either::First(c) => m.handle(c).await,
                Either::Second(_) => {}
            }
        }
    }
}

/// Minimal SNTP client: returns Unix time in seconds (UTC).
async fn sntp(stack: Stack<'static>) -> Option<i64> {
    let addrs = with_timeout(Duration::from_secs(5), stack.dns_query("pool.ntp.org", DnsQueryType::A)).await.ok()?.ok()?;
    let addr = *addrs.first()?;
    let mut rxm = [PacketMetadata::EMPTY; 2];
    let mut txm = [PacketMetadata::EMPTY; 2];
    let mut rxb = [0u8; 128];
    let mut txb = [0u8; 128];
    let mut sock = UdpSocket::new(stack, &mut rxm, &mut rxb, &mut txm, &mut txb);
    sock.bind(0).ok()?;
    for _ in 0..3 {
        let mut pkt = [0u8; 48];
        pkt[0] = 0x1B;
        sock.send_to(&pkt, (addr, 123)).await.ok()?;
        let mut resp = [0u8; 64];
        if let Ok(Ok((n, _))) = with_timeout(Duration::from_secs(2), sock.recv_from(&mut resp)).await {
            if n >= 48 {
                let secs = u32::from_be_bytes([resp[40], resp[41], resp[42], resp[43]]) as i64;
                if secs > 2_208_988_800 + 1_700_000_000 - 1_700_000_000 {
                    return Some(secs - 2_208_988_800);
                }
            }
        }
    }
    None
}

// ───────────────────────────── OTA over HTTP ─────────────────────────────

const PAGE: &str = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<html><body style='font-family:monospace;background:#001;color:#4af'>\
<h2>Aetheris OS &middot; OTA</h2><p>Seleste Technologies</p>\
<p>Upload firmware: <code>curl --data-binary @aetheris.bin http://THIS-IP/ota</code></p></body></html>";

async fn respond(sock: &mut TcpSocket<'_>, code: &str, body: &str) {
    let _ = sock.write_all(b"HTTP/1.1 ").await;
    let _ = sock.write_all(code.as_bytes()).await;
    let _ = sock.write_all(b"\r\nConnection: close\r\nContent-Type: text/plain\r\n\r\n").await;
    let _ = sock.write_all(body.as_bytes()).await;
    let _ = sock.flush().await;
}

fn find(h: &[u8], needle: &[u8]) -> Option<usize> {
    h.windows(needle.len()).position(|w| w == needle)
}

#[embassy_executor::task]
pub async fn http_task(stack: Stack<'static>) {
    let mut rx = [0u8; 2048];
    let mut tx = [0u8; 1024];
    loop {
        if !OTA_ON.load(Ordering::Relaxed) {
            Timer::after(Duration::from_millis(500)).await;
            continue;
        }
        let mut sock = TcpSocket::new(stack, &mut rx, &mut tx);
        sock.set_timeout(Some(Duration::from_secs(15)));
        match select(sock.accept(80), Timer::after(Duration::from_secs(1))).await {
            Either::First(Ok(())) => {}
            _ => {
                sock.abort();
                continue;
            }
        }
        serve(&mut sock).await;
        sock.close();
        let _ = with_timeout(Duration::from_secs(2), sock.flush()).await;
        sock.abort();
    }
}

async fn serve(sock: &mut TcpSocket<'_>) {
    let mut hdr = [0u8; 1024];
    let mut n = 0usize;
    let hend = loop {
        match with_timeout(Duration::from_secs(5), sock.read(&mut hdr[n..])).await {
            Ok(Ok(k)) if k > 0 => n += k,
            _ => return,
        }
        if let Some(p) = find(&hdr[..n], b"\r\n\r\n") {
            break p + 4;
        }
        if n >= hdr.len() {
            respond(sock, "431 Too Large", "headers too large\n").await;
            return;
        }
    };
    let text = core::str::from_utf8(&hdr[..hend]).unwrap_or("");
    let first = text.lines().next().unwrap_or("");
    if !(first.starts_with("POST /ota")) {
        let _ = sock.write_all(PAGE.as_bytes()).await;
        return;
    }
    let mut total = 0u32;
    for l in text.lines() {
        let low = l.as_bytes();
        if low.len() > 15 && low[..15].eq_ignore_ascii_case(b"content-length:") {
            total = l[15..].trim().parse().unwrap_or(0);
        }
    }
    if total < 1024 {
        respond(sock, "400 Bad Request", "missing or tiny content-length\n").await;
        return;
    }
    let first_body = &hdr[hend..n];
    match ota_write(sock, total, first_body).await {
        Ok(()) => {
            upd(|s| s.ota = OtaState::Done);
            respond(sock, "200 OK", "OK - rebooting into new firmware\n").await;
            Timer::after(Duration::from_millis(1500)).await;
            esp_hal::system::software_reset();
        }
        Err(e) => {
            upd(|s| s.ota = OtaState::Failed(e));
            respond(sock, "500 Error", e).await;
        }
    }
}

/// Picks the OTA slot that is *not* running, streams the image into it, then points `otadata` at it.
///
/// Slot selection and `otadata` handling are done here instead of via `OtaUpdater`: on a board whose
/// `otadata` holds leftover bytes (e.g. from an older partition layout) `OtaUpdater` refuses to work
/// ("no free OTA slot"), and with blank `otadata` it can pick the running slot or write sequence 0.
async fn ota_write(sock: &mut TcpSocket<'_>, total: u32, first: &[u8]) -> Result<(), &'static str> {
    let mut g = FLASH.lock().await;
    let flash = g.as_mut().ok_or("no flash")?;
    let mut ptbuf = [0u8; PARTITION_TABLE_MAX_LEN];
    let pt = read_partition_table(flash, &mut ptbuf).map_err(|_| "no partition table")?;
    let app = |st| pt.find_partition(PartitionType::App(st)).ok().flatten();
    let (Some(p0), Some(p1)) = (app(AppPartitionSubType::Ota0), app(AppPartitionSubType::Ota1)) else {
        return Err("no OTA partitions (flash with partitions.csv)");
    };
    let otadata = pt.find_partition(PartitionType::Data(DataPartitionSubType::Ota)).ok().flatten().ok_or("no otadata partition")?;
    // never write the slot we are running from: refuse if it cannot be determined
    let booted = pt.booted_partition().ok().flatten().ok_or("running slot unknown")?.offset();
    let (target, target_idx) = if booted == p0.offset() { (p1, 1u32) } else { (p0, 0u32) };
    log::info!("OTA: running at {:#x}, writing slot ota_{}", booted, target_idx);
    {
        let mut region = target.as_flash_region(flash);
        if total as usize > region.partition_size() {
            return Err("image larger than slot");
        }
        let mut chunk = [0xFFu8; 4096];
        let mut fill = 0usize;
        let mut off = 0u32;
        let mut done = 0u32;
        let mut tmp = [0u8; 1024];
        let mut feed_first = true;
        loop {
            let data: &[u8] = if feed_first {
                feed_first = false;
                first
            } else {
                if done >= total {
                    break;
                }
                match with_timeout(Duration::from_secs(10), sock.read(&mut tmp)).await {
                    Ok(Ok(k)) if k > 0 => &tmp[..k],
                    _ => return Err("connection lost"),
                }
            };
            let mut i = 0;
            while i < data.len() {
                let take = (4096 - fill).min(data.len() - i);
                chunk[fill..fill + take].copy_from_slice(&data[i..i + take]);
                fill += take;
                i += take;
                done += take as u32;
                if fill == 4096 {
                    if off == 0 && !(chunk[0] == 0xE9 && chunk[12] == 0x0D && chunk[13] == 0x00) {
                        return Err("not an ESP32-C6 image");
                    }
                    region.erase(off, off + 4096).map_err(|_| "erase failed")?;
                    region.write(off, &chunk).map_err(|_| "write failed")?;
                    off += 4096;
                    fill = 0;
                    chunk = [0xFF; 4096];
                }
            }
            upd(|s| s.ota = OtaState::Receiving { done, total });
            if done >= total {
                break;
            }
        }
        if fill > 0 {
            if off == 0 && !(chunk[0] == 0xE9 && chunk[12] == 0x0D && chunk[13] == 0x00) {
                return Err("not an ESP32-C6 image");
            }
            region.erase(off, off + 4096).map_err(|_| "erase failed")?;
            region.write(off, &chunk).map_err(|_| "write failed")?;
        }
    }
    // otadata = two 4 KB sectors, each starting with a 32-byte ESP-IDF `esp_ota_select_entry_t`.
    // The bootloader boots slot (seq - 1) % 2 of the highest valid sequence number.
    let mut od = otadata.as_flash_region(flash);
    let mut e = [[0u8; 32]; 2];
    od.read(0, &mut e[0]).map_err(|_| "otadata read failed")?;
    od.read(0x1000, &mut e[1]).map_err(|_| "otadata read failed")?;
    let seqs = [ota_entry_seq(&e[0]), ota_entry_seq(&e[1])];
    let max = seqs.iter().flatten().copied().max().unwrap_or(0);
    let mut seq = max + 1;
    while (seq - 1) % 2 != target_idx {
        seq += 1;
    }
    // keep the entry that boots the current image as a fallback, overwrite the other one
    let w = if seqs[0] == Some(max) && max != 0 { 1 } else { 0 };
    let mut ent = [0xFFu8; 32];
    ent[0..4].copy_from_slice(&seq.to_le_bytes());
    ent[24..28].copy_from_slice(&(OtaImageState::New as u32).to_le_bytes());
    ent[28..32].copy_from_slice(&esp_hal::rom::crc::crc32_le(u32::MAX, &seq.to_le_bytes()).to_le_bytes());
    od.write(w as u32 * 0x1000, &ent).map_err(|_| "otadata write failed")?;
    // leftover garbage in the other entry would make the boot-time "mark valid" step fail: blank it
    let other = 1 - w;
    let blank = e[other][0..4] == [0xFF; 4] && e[other][24..28] == [0xFF; 4];
    if seqs[other].is_none() && !blank {
        od.write(other as u32 * 0x1000, &[0xFF; 32]).map_err(|_| "otadata write failed")?;
    }
    log::info!("OTA: otadata seq {} -> boot ota_{}", seq, target_idx);
    Ok(())
}

/// Sequence number of a valid, initialised otadata entry.
fn ota_entry_seq(e: &[u8; 32]) -> Option<u32> {
    let seq = u32::from_le_bytes([e[0], e[1], e[2], e[3]]);
    let crc = u32::from_le_bytes([e[28], e[29], e[30], e[31]]);
    (seq != u32::MAX && crc == esp_hal::rom::crc::crc32_le(u32::MAX, &e[0..4])).then_some(seq)
}
