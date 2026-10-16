use super::shared::WIFI_STATE;
use super::storage::FLASH;
use crate::sys::{ProgramState, ProgramTarget};
use embassy_net::tcp::TcpSocket;
use embassy_time::{Duration, Timer, with_timeout};
use embedded_io_async::Write;
use esp_bootloader_esp_idf::partitions::{PARTITION_TABLE_MAX_LEN, read_partition_table};
use esp_hal::Async;
use esp_hal::gpio::{Level, Output};
use esp_hal::uart::{RxError, Uart};
use sha2::{Digest, Sha256};

pub const STAGING_OFFSET: u32 = 0x620000;
pub const STAGING_SIZE: u32 = 0x300000;
const APP_OFFSET: u32 = 0x10000;
const ERASE_SIZE: u32 = 0x1000;
const BLOCK_SIZE: usize = 1024;
const SLIP_END: u8 = 0xC0;
const SLIP_ESC: u8 = 0xDB;

fn set_state(state: ProgramState) {
    WIFI_STATE.lock(|s| s.borrow_mut().program = state);
}

pub async fn stage_image(
    sock: &mut TcpSocket<'_>,
    total: u32,
    first: &[u8],
    target: ProgramTarget,
) -> Result<(), &'static str> {
    if total < 1024 || total > STAGING_SIZE || first.len() > total as usize {
        return Err("image size out of range");
    }
    if esp_storage::flash_encryption() {
        return Err("host flash encryption prevents safe staging");
    }

    let mut guard = FLASH.lock().await;
    let flash = guard.as_mut().ok_or("host flash unavailable")?;
    if flash.capacity() < (STAGING_OFFSET + STAGING_SIZE) as usize {
        return Err("host flash too small for staging");
    }
    let mut ptbuf = [0u8; PARTITION_TABLE_MAX_LEN];
    read_partition_table(flash, &mut ptbuf).map_err(|_| "cannot verify host partitions")?;
    if !staging_is_free(&ptbuf) {
        return Err("staging area overlaps a host partition");
    }

    let mut block = [0xFFu8; ERASE_SIZE as usize];
    let mut fill = 0usize;
    let mut offset = 0u32;
    let mut done = 0u32;
    let mut input = [0u8; 1024];
    let mut use_first = true;

    loop {
        let data = if use_first {
            use_first = false;
            first
        } else {
            if done >= total {
                break;
            }
            let remaining = (total - done) as usize;
            let want = remaining.min(input.len());
            match with_timeout(Duration::from_secs(15), sock.read(&mut input[..want])).await {
                Ok(Ok(n)) if n > 0 => &input[..n],
                _ => return Err("upload connection lost"),
            }
        };
        let mut pos = 0;
        while pos < data.len() {
            let n = (block.len() - fill).min(data.len() - pos);
            block[fill..fill + n].copy_from_slice(&data[pos..pos + n]);
            fill += n;
            pos += n;
            done += n as u32;
            if fill == block.len() {
                flash
                    .erase(
                        STAGING_OFFSET + offset,
                        STAGING_OFFSET + offset + ERASE_SIZE,
                    )
                    .map_err(|_| "staging erase failed")?;
                flash
                    .write_nor(STAGING_OFFSET + offset, &block)
                    .map_err(|_| "staging write failed")?;
                offset += ERASE_SIZE;
                fill = 0;
                block.fill(0xFF);
            }
        }
        set_state(ProgramState::Staging { done, total });
        if done >= total {
            break;
        }
    }

    if fill > 0 {
        let erase_end = (fill as u32 + ERASE_SIZE - 1) & !(ERASE_SIZE - 1);
        flash
            .erase(STAGING_OFFSET + offset, STAGING_OFFSET + offset + erase_end)
            .map_err(|_| "staging erase failed")?;
        let write_len = (fill + 3) & !3;
        flash
            .write_nor(STAGING_OFFSET + offset, &block[..write_len])
            .map_err(|_| "staging write failed")?;
    }

    validate_image(flash, total, target)
}

fn validate_image(
    flash: &mut esp_storage::FlashStorage<'static>,
    total: u32,
    target: ProgramTarget,
) -> Result<(), &'static str> {
    let mut header = [0u8; 24];
    flash
        .read(STAGING_OFFSET, &mut header)
        .map_err(|_| "cannot read staged image")?;
    let chip_id = u16::from_le_bytes([header[12], header[13]]);
    if header[0] != 0xE9
        || !(1..=16).contains(&header[1])
        || header[23] > 1
        || chip_id != target.chip_id()
    {
        return Err("invalid image or target chip mismatch");
    }

    let mut cursor = 24u32;
    let mut checksum = 0xEFu8;
    let mut segment_data = [0u8; 512];
    for _ in 0..header[1] {
        if cursor.checked_add(8).filter(|end| *end <= total).is_none() {
            return Err("truncated image segment header");
        }
        let mut segment = [0u8; 8];
        flash
            .read(STAGING_OFFSET + cursor, &mut segment)
            .map_err(|_| "invalid image segments")?;
        let size = u32::from_le_bytes([segment[4], segment[5], segment[6], segment[7]]);
        if size % 4 != 0 || size >= 16 * 1024 * 1024 {
            return Err("invalid image segment size");
        }
        cursor = cursor.checked_add(8).ok_or("invalid image segments")?;
        let end = cursor.checked_add(size).ok_or("invalid image segments")?;
        if end > total {
            return Err("truncated image segment");
        }
        while cursor < end {
            let n = (end - cursor).min(segment_data.len() as u32) as usize;
            flash
                .read(STAGING_OFFSET + cursor, &mut segment_data[..n])
                .map_err(|_| "cannot verify image")?;
            for byte in &segment_data[..n] {
                checksum ^= byte;
            }
            cursor += n as u32;
        }
    }

    let image_end = cursor.checked_add(16).ok_or("invalid image length")? & !15;
    let checksum_offset = image_end.checked_sub(1).ok_or("invalid image length")?;
    if image_end > total {
        return Err("truncated image checksum");
    }
    let mut stored_checksum = [0u8; 1];
    flash
        .read(STAGING_OFFSET + checksum_offset, &mut stored_checksum)
        .map_err(|_| "cannot read image checksum")?;
    if stored_checksum[0] != checksum {
        return Err("image checksum mismatch");
    }

    if header[23] != 0 {
        let expected_end = image_end.checked_add(32).ok_or("invalid image length")?;
        if expected_end != total {
            return Err("not an app-only image");
        }
        let mut hasher = Sha256::new();
        let mut offset = 0u32;
        let mut chunk = [0u8; 1024];
        while offset < image_end {
            let n = (image_end - offset).min(chunk.len() as u32) as usize;
            flash
                .read(STAGING_OFFSET + offset, &mut chunk[..n])
                .map_err(|_| "cannot verify image hash")?;
            hasher.update(&chunk[..n]);
            offset += n as u32;
        }
        let digest = hasher.finalize();
        let mut appended = [0u8; 32];
        flash
            .read(STAGING_OFFSET + image_end, &mut appended)
            .map_err(|_| "cannot read image hash")?;
        if digest.as_slice() != appended {
            return Err("image hash mismatch");
        }
    } else if image_end != total {
        return Err("not an app-only image");
    }
    Ok(())
}

fn staging_is_free(table: &[u8]) -> bool {
    let stage_end = STAGING_OFFSET + STAGING_SIZE;
    let mut found_end = false;
    for entry in table.chunks_exact(32) {
        let magic = u16::from_le_bytes([entry[0], entry[1]]);
        if magic == 0xEBEB || magic == 0xFFFF {
            found_end = true;
            break;
        }
        if magic != 0x50AA {
            return false;
        }
        let start = u32::from_le_bytes([entry[4], entry[5], entry[6], entry[7]]);
        let size = u32::from_le_bytes([entry[8], entry[9], entry[10], entry[11]]);
        let Some(end) = start.checked_add(size) else {
            return false;
        };
        if start < stage_end && end > STAGING_OFFSET {
            return false;
        }
    }
    found_end
}

pub async fn flash_staged(
    uart: &mut Uart<'static, Async>,
    boot: &mut Output<'static>,
    reset: &mut Output<'static>,
    target: ProgramTarget,
    total: u32,
) -> Result<(), &'static str> {
    set_state(ProgramState::Connecting);
    boot.set_level(Level::Low);
    reset.set_level(Level::Low);
    Timer::after(Duration::from_millis(100)).await;
    reset.set_level(Level::High);
    Timer::after(Duration::from_millis(100)).await;

    let result = flash_image(uart, target, total).await;

    boot.set_level(Level::High);
    reset.set_level(Level::Low);
    Timer::after(Duration::from_millis(50)).await;
    reset.set_level(Level::High);
    Timer::after(Duration::from_millis(500)).await;
    result
}

async fn flash_image(
    uart: &mut Uart<'static, Async>,
    target: ProgramTarget,
    total: u32,
) -> Result<(), &'static str> {
    let mut reader = UartFrameReader::new();
    let sync_payload = [
        0x07, 0x07, 0x12, 0x20, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
        0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
        0x55, 0x55, 0x55, 0x55, 0x55, 0x55,
    ];
    send_command(uart, 0x08, &sync_payload, 0).await?;
    Timer::after(Duration::from_millis(10)).await;
    let mut synced = false;
    for _ in 0..16 {
        let reply = with_timeout(Duration::from_secs(5), read_frame(uart, &mut reader))
            .await
            .map_err(|_| "no ESP ROM sync reply; check BOOT, EN, RX/TX, and GND")??;
        if valid_response(&reply, 0x08) {
            let status_len = if reply.len() == 10 || reply.len() == 26 {
                2
            } else {
                4
            };
            if reply.len() < 4 + status_len || reply[reply.len() - status_len..][..2] != [0, 0] {
                return Err("ESP ROM rejected sync");
            }
            synced = true;
            break;
        }
    }
    if !synced {
        return Err("ESP ROM did not acknowledge sync");
    }
    let chip_magic = command(uart, &mut reader, 0x0A, &0x40001000u32.to_le_bytes(), 0, 5).await?;
    let magic = u32::from_le_bytes(
        chip_magic
            .get(..4)
            .ok_or("cannot identify slave chip")?
            .try_into()
            .map_err(|_| "cannot identify slave chip")?,
    );
    if !chip_matches(target, magic) {
        return Err("attached slave chip does not match selection");
    }
    command(uart, &mut reader, 0x0D, &[0; 8], 0, 5).await?;

    let blocks = total.div_ceil(BLOCK_SIZE as u32);
    let erase_size = total.div_ceil(ERASE_SIZE) * ERASE_SIZE;
    let mut begin = [0u8; 20];
    begin[0..4].copy_from_slice(&erase_size.to_le_bytes());
    begin[4..8].copy_from_slice(&blocks.to_le_bytes());
    begin[8..12].copy_from_slice(&(BLOCK_SIZE as u32).to_le_bytes());
    begin[12..16].copy_from_slice(&APP_OFFSET.to_le_bytes());
    command(uart, &mut reader, 0x02, &begin, 0, 120).await?;

    let mut block = [0xFFu8; BLOCK_SIZE];
    for seq in 0..blocks {
        block.fill(0xFF);
        let offset = seq * BLOCK_SIZE as u32;
        let len = ((total - offset) as usize).min(BLOCK_SIZE);
        {
            let mut guard = FLASH.lock().await;
            let flash = guard.as_mut().ok_or("host flash unavailable")?;
            flash
                .read(STAGING_OFFSET + offset, &mut block[..len])
                .map_err(|_| "staged image read failed")?;
        }
        let mut payload = [0u8; BLOCK_SIZE + 16];
        payload[0..4].copy_from_slice(&(BLOCK_SIZE as u32).to_le_bytes());
        payload[4..8].copy_from_slice(&seq.to_le_bytes());
        payload[16..].copy_from_slice(&block);
        let checksum = block.iter().fold(0xEFu8, |sum, byte| sum ^ byte);
        command(uart, &mut reader, 0x03, &payload, checksum as u32, 5).await?;
        set_state(ProgramState::Flashing {
            done: (offset + len as u32).min(total),
            total,
        });
    }
    command(uart, &mut reader, 0x04, &[1, 0, 0, 0], 0, 5).await?;
    Ok(())
}

fn chip_matches(target: ProgramTarget, magic: u32) -> bool {
    match target {
        ProgramTarget::Esp32C3 => {
            [0x6921_506F, 0x1B31_506F, 0x4881_606F, 0x4361_606F].contains(&magic)
        }
        ProgramTarget::Esp32S3 => magic == 0x0000_0009,
        ProgramTarget::Esp32C6 => magic == 0x2CE0_806F,
    }
}

async fn command(
    uart: &mut Uart<'static, Async>,
    reader: &mut UartFrameReader,
    cmd: u8,
    payload: &[u8],
    checksum: u32,
    timeout_secs: u64,
) -> Result<alloc::vec::Vec<u8>, &'static str> {
    send_command(uart, cmd, payload, checksum).await?;

    for _ in 0..16 {
        let reply = with_timeout(Duration::from_secs(timeout_secs), read_frame(uart, reader))
            .await
            .map_err(|_| match cmd {
                0x0A => "slave chip ID query timeout",
                0x0D => "slave flash attach timeout",
                0x02 => "slave flash begin timeout",
                0x03 => "slave flash data timeout",
                0x04 => "slave flash finish timeout",
                _ => "slave bootloader command timeout",
            })??;
        if !valid_response(&reply, cmd) {
            continue;
        }
        let status_len = if reply.len() == 10 || reply.len() == 26 {
            2
        } else {
            4
        };
        if reply.len() < 4 + status_len || reply[reply.len() - status_len..][..2] != [0, 0] {
            return Err("slave bootloader rejected command");
        }
        return Ok(reply[4..reply.len() - status_len].into());
    }
    Err("slave bootloader response mismatch")
}

async fn send_command(
    uart: &mut Uart<'static, Async>,
    cmd: u8,
    payload: &[u8],
    checksum: u32,
) -> Result<(), &'static str> {
    let mut frame = [0u8; 2200];
    let mut len = 1usize;
    frame[0] = SLIP_END;
    slip_push(&mut frame, &mut len, 0x00)?;
    slip_push(&mut frame, &mut len, cmd)?;
    slip_push(&mut frame, &mut len, payload.len() as u8)?;
    slip_push(&mut frame, &mut len, (payload.len() >> 8) as u8)?;
    for b in checksum.to_le_bytes() {
        slip_push(&mut frame, &mut len, b)?;
    }
    for &b in payload {
        slip_push(&mut frame, &mut len, b)?;
    }
    frame[len] = SLIP_END;
    len += 1;
    uart.write_all(&frame[..len])
        .await
        .map_err(|_| "slave UART write failed")?;
    Write::flush(uart)
        .await
        .map_err(|_| "slave UART flush failed")?;
    Ok(())
}

fn valid_response(reply: &[u8], cmd: u8) -> bool {
    reply.len() >= 10 && reply[0] == 1 && reply[1] == cmd
}

fn slip_push(frame: &mut [u8], len: &mut usize, byte: u8) -> Result<(), &'static str> {
    let encoded: &[u8] = match byte {
        SLIP_END => &[SLIP_ESC, 0xDC],
        SLIP_ESC => &[SLIP_ESC, 0xDD],
        _ => core::slice::from_ref(&byte),
    };
    if *len + encoded.len() >= frame.len() {
        return Err("slave command too large");
    }
    frame[*len..*len + encoded.len()].copy_from_slice(encoded);
    *len += encoded.len();
    Ok(())
}

struct UartFrameReader {
    pending: [u8; 128],
    pos: usize,
    len: usize,
}

impl UartFrameReader {
    const fn new() -> Self {
        Self {
            pending: [0; 128],
            pos: 0,
            len: 0,
        }
    }
}

async fn read_frame(
    uart: &mut Uart<'static, Async>,
    reader: &mut UartFrameReader,
) -> Result<alloc::vec::Vec<u8>, &'static str> {
    let mut encoded = [0u8; 2200];
    let mut len = 0usize;
    let mut started = false;
    loop {
        if reader.pos == reader.len {
            reader.len =
                uart.read_async(&mut reader.pending)
                    .await
                    .map_err(|error| match error {
                        RxError::FifoOverflowed => "slave UART RX overflow; check wiring",
                        RxError::GlitchOccurred => "slave UART RX glitch; check wiring and ground",
                        RxError::FrameFormatViolated => {
                            "slave UART framing error; check RX/TX and baud"
                        }
                        RxError::ParityMismatch => "slave UART parity error",
                        _ => "slave UART read error",
                    })?;
            reader.pos = 0;
        }
        if reader.len == 0 {
            continue;
        }
        let byte = reader.pending[reader.pos];
        reader.pos += 1;
        if byte == SLIP_END {
            if started && len > 0 {
                break;
            }
            started = true;
            continue;
        }
        if started {
            if len == encoded.len() {
                return Err("slave response too large");
            }
            encoded[len] = byte;
            len += 1;
        }
    }

    let mut decoded = alloc::vec::Vec::with_capacity(len);
    let mut i = 0;
    while i < len {
        if encoded[i] == SLIP_ESC {
            i += 1;
            if i == len {
                return Err("invalid slave SLIP frame");
            }
            decoded.push(match encoded[i] {
                0xDC => SLIP_END,
                0xDD => SLIP_ESC,
                _ => return Err("invalid slave SLIP escape"),
            });
        } else {
            decoded.push(encoded[i]);
        }
        i += 1;
    }
    Ok(decoded)
}

pub fn state_error(message: &'static str) {
    set_state(ProgramState::Failed(message));
}

pub fn state_done() {
    set_state(ProgramState::Done);
}
