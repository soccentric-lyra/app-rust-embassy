//! Factory provisioning (PRV1) record reader (A-03).
//!
//! The factory writes the record into bank 2 sector 13 (0x0810_4000, or
//! 0x0800_4000 while bank 2 is swapped in).  The reader checks the header,
//! both CRCs and the UID binding, then returns the identity as the
//! HelloReply detail string for the factory boot check.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - Date: 2026-11-04
//! - SRS: factory identity contract PRV1, A-02/A-03/A-05

use core::fmt::Write;

use crate::board;
use crate::protocol::frame::crc32;

/// Record magic, "PRV1" little-endian.
const MAGIC: u32 = 0x3156_5250;
/// Supported header format version.
const FORMAT: u16 = 1;
/// Header size in bytes, ending with its CRC-32.
const HEADER_SIZE: usize = 32;
/// Largest accepted TLV area in bytes.
const MAX_TLV: usize = 4096;
/// Record address with bank 2 at its normal place.
const ADDR_A: u32 = 0x0810_4000;
/// Record address while bank 2 is swapped in.
const ADDR_SWAPPED: u32 = 0x0800_4000;

/// TLV tag: serial number.
const T_SERIAL: u8 = 0x01;
/// TLV tag: product name.
const T_PRODUCT: u8 = 0x02;
/// TLV tag: hardware revision.
const T_HW_REV: u8 = 0x03;
/// TLV tag: build profile.
const T_PROFILE: u8 = 0x06;
/// TLV tag: 6-byte MAC address.
const T_MAC: u8 = 0x10;

/// Identity fields borrowed from the record in flash.
#[derive(Default)]
struct Identity<'a> {
    /// Record revision from the header.
    revision: u16,
    /// Serial number (required).
    serial: &'a [u8],
    /// Product name (required).
    product: &'a [u8],
    /// Hardware revision.
    hw_revision: &'a [u8],
    /// Build profile.
    profile: &'a [u8],
    /// MAC address, if present with 6 bytes.
    mac: Option<[u8; 6]>,
}

/// Fixed buffer the detail string is formatted into.
struct Buffer<'a> {
    /// Destination bytes.
    data: &'a mut [u8],
    /// Bytes used in `data`.
    len: usize,
}

impl Write for Buffer<'_> {
    /// Append `text` whole, or fail without writing if it does not fit with
    /// one byte to spare.
    fn write_str(&mut self, text: &str) -> core::fmt::Result {
        let bytes = text.as_bytes();
        if self.len + bytes.len() >= self.data.len() {
            return Err(core::fmt::Error);
        }
        self.data[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

/// Read a flash word.
fn word(address: u32) -> u32 {
    // SAFETY: fixed flash addresses inside the F767 address map.
    unsafe { core::ptr::read_volatile(address as *const u32) }
}

/// Borrow `len` flash bytes at `address`.
fn bytes(address: u32, len: usize) -> &'static [u8] {
    // SAFETY: the caller bounds len to the 16 KB identity sector.
    unsafe { core::slice::from_raw_parts(address as *const u8, len) }
}

/// Invalidate [`MAX_TLV`] bytes of D-cache from `address`.
fn invalidate(address: u32) {
    // SAFETY: the identity sector is not shared with live cached data.
    let mut core = unsafe { cortex_m::Peripherals::steal() };
    // SAFETY: flash is read-only here, so no dirty line is discarded.
    unsafe {
        core.SCB
            .invalidate_dcache_by_address(address as usize, MAX_TLV)
    };
}

/// Return the 96-bit UID as 12 little-endian bytes.
fn uid_words() -> [u8; 12] {
    let mut uid = [0u8; 12];
    for (index, slot) in uid.as_chunks_mut::<4>().0.iter_mut().enumerate() {
        // SAFETY: the UID is three read-only words at a fixed system address.
        let value = unsafe { core::ptr::read_volatile(board::UID_BASE.add(index)) };
        slot.copy_from_slice(&value.to_le_bytes());
    }
    uid
}

/// Parse and validate the record at `address`.
///
/// Returns `None` for a bad magic, format, length, CRC, UID binding or TLV,
/// or when the serial or product is missing.
fn parse(address: u32) -> Option<Identity<'static>> {
    invalidate(address);
    if word(address) != MAGIC {
        return None;
    }
    let header = bytes(address, HEADER_SIZE);
    let format = u16::from_le_bytes([header[4], header[5]]);
    let revision = u16::from_le_bytes([header[6], header[7]]);
    let length = u32::from_le_bytes([header[8], header[9], header[10], header[11]]) as usize;
    let tlv_crc = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);
    let header_crc = u32::from_le_bytes([header[28], header[29], header[30], header[31]]);
    if format != FORMAT || !(2..=MAX_TLV).contains(&length) || crc32(&header[..28]) != header_crc {
        return None;
    }
    if header[16..28] != uid_words() {
        return None;
    }
    let tlv = bytes(address + HEADER_SIZE as u32, length);
    if crc32(tlv) != tlv_crc {
        return None;
    }

    let mut identity = Identity {
        revision,
        ..Identity::default()
    };
    let mut offset = 0usize;
    while offset + 2 <= tlv.len() {
        let kind = tlv[offset];
        let size = tlv[offset + 1] as usize;
        offset += 2;
        if kind == 0 {
            break;
        }
        if offset + size > tlv.len() {
            return None;
        }
        let value = &tlv[offset..offset + size];
        match kind {
            T_SERIAL => identity.serial = value,
            T_PRODUCT => identity.product = value,
            T_HW_REV => identity.hw_revision = value,
            T_PROFILE => identity.profile = value,
            T_MAC if size == 6 => {
                let mut mac = [0u8; 6];
                mac.copy_from_slice(value);
                identity.mac = Some(mac);
            }
            _ => {}
        }
        offset += size;
    }
    if identity.serial.is_empty() || identity.product.is_empty() {
        return None;
    }
    Some(identity)
}

/// Return `value` as UTF-8, or "" if invalid.
fn text(value: &[u8]) -> &str {
    core::str::from_utf8(value).unwrap_or("")
}

/// Format the HelloReply detail into `buf` and return it.
///
/// Tries [`ADDR_A`] then [`ADDR_SWAPPED`]; returns "no identity" when neither
/// holds a valid record.  Text that does not fit in `buf` is dropped.
pub fn detail(buf: &mut [u8]) -> &str {
    let identity = parse(ADDR_A).or_else(|| parse(ADDR_SWAPPED));
    let mut out = Buffer { data: buf, len: 0 };
    match identity {
        Some(identity) => {
            let _ = write!(out, "PRV1 serial={};mac=", text(identity.serial));
            if let Some(mac) = identity.mac {
                let _ = write!(
                    out,
                    "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
                    mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
                );
            }
            let _ = write!(
                out,
                ";hw={};product={};profile={};revision={}",
                text(identity.hw_revision),
                text(identity.product),
                text(identity.profile),
                identity.revision
            );
        }
        None => {
            let _ = write!(out, "no identity");
        }
    }
    core::str::from_utf8(&out.data[..out.len]).unwrap_or("no identity")
}
