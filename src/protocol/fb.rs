//! Fixed-buffer FlatBuffers reader and writer.
//!
//! OPEN-R01: the upstream Rust flatbuffers runtime needs `alloc` and RDEMO
//! links no heap, so this module implements the wire format for the small
//! `protocol/hwlink.fbs` message set.  It stays behind the protocol module
//! so generated code can replace it once a no-alloc generator exists.
//!
//! - Author: Sandesh Ghimire <sandesh@soccentric.com>
//! - SRS: RLNK-002 (fixed-size buffer, no heap)
#![allow(dead_code)]

/// FlatBuffers file identifier of HWLINK messages.
pub const FILE_ID: &[u8; 4] = b"HWLK";

/// Table field value for [`Builder::table`].
#[derive(Clone, Copy)]
pub enum Value {
    /// `ubyte`/`bool` scalar.
    U8(u8),
    /// `uint16` scalar.
    U16(u16),
    /// `uint32` scalar.
    U32(u32),
    /// `ulong` scalar.
    U64(u64),
    /// `int32` scalar.
    I32(i32),
    /// Position in the buffer of a string, vector or table.
    Offset(usize),
}

impl Value {
    /// Alignment in bytes.
    fn align(self) -> usize {
        match self {
            Value::U8(_) => 1,
            Value::U16(_) => 2,
            Value::U32(_) | Value::I32(_) | Value::Offset(_) => 4,
            Value::U64(_) => 8,
        }
    }

    /// Encoded size in bytes.
    fn size(self) -> usize {
        match self {
            Value::U8(_) => 1,
            Value::U16(_) => 2,
            Value::U32(_) | Value::I32(_) | Value::Offset(_) => 4,
            Value::U64(_) => 8,
        }
    }

    /// Write the value little-endian at `at`; offsets become relative to
    /// `at`.
    fn write(self, out: &mut [u8], at: usize) {
        match self {
            Value::U8(v) => out[at] = v,
            Value::U16(v) => out[at..at + 2].copy_from_slice(&v.to_le_bytes()),
            Value::U32(v) => out[at..at + 4].copy_from_slice(&v.to_le_bytes()),
            Value::U64(v) => out[at..at + 8].copy_from_slice(&v.to_le_bytes()),
            Value::I32(v) => out[at..at + 4].copy_from_slice(&v.to_le_bytes()),
            Value::Offset(target) => {
                let rel = (target as u64).wrapping_sub(at as u64) as u32;
                out[at..at + 4].copy_from_slice(&rel.to_le_bytes());
            }
        }
    }
}

/// Round `v` up to a multiple of `a` (a power of two).
#[inline]
const fn align_up(v: usize, a: usize) -> usize {
    (v + a - 1) & !(a - 1)
}

/// Round `v` down to a multiple of `a` (a power of two).
#[inline]
const fn align_down(v: usize, a: usize) -> usize {
    v & !(a - 1)
}

/// Back-to-front FlatBuffers builder over a caller buffer.
///
/// Every method that reserves space panics when `buf` is exhausted.
pub struct Builder<'a> {
    /// Output buffer, filled from the end.
    buf: &'a mut [u8],
    /// Start of the used region.
    head: usize,
}

impl<'a> Builder<'a> {
    /// Start a builder over `buf`, zeroing it.
    pub fn new(buf: &'a mut [u8]) -> Self {
        buf.fill(0);
        let head = buf.len();
        Self { buf, head }
    }

    /// Reserve `size` bytes at `align` below the head; return their start.
    ///
    /// # Panics
    ///
    /// Panics when the buffer is exhausted.
    fn take(&mut self, size: usize, align: usize) -> usize {
        assert!(size <= self.head, "FlatBuffers buffer exhausted");
        let start = align_down(self.head - size, align);
        self.head = start;
        start
    }

    /// Add a NUL-terminated string; return its position.
    ///
    /// # Panics
    ///
    /// Panics when the buffer is exhausted.
    pub fn string(&mut self, s: &str) -> usize {
        let bytes = s.as_bytes();
        let start = self.take(4 + bytes.len() + 1, 4);
        self.buf[start..start + 4].copy_from_slice(&(bytes.len() as u32).to_le_bytes());
        self.buf[start + 4..start + 4 + bytes.len()].copy_from_slice(bytes);
        self.buf[start + 4 + bytes.len()] = 0;
        start
    }

    /// Add a table and its vtable from `(field id, value)` pairs; return the
    /// table position.
    ///
    /// # Panics
    ///
    /// Panics when the buffer is exhausted or a field id is 64 or more.
    pub fn table(&mut self, fields: &[(u8, Value)]) -> usize {
        let mut offsets = [0u16; 64];
        let mut max_id = 0usize;
        let mut table_size = 4usize;
        let mut max_align = 4usize;

        for &(id, value) in fields {
            let id = id as usize;
            assert!(id < offsets.len(), "too many FlatBuffers fields");
            max_id = max_id.max(id);
            max_align = max_align.max(value.align());
            table_size = align_up(table_size, value.align());
            offsets[id] = table_size as u16;
            table_size += value.size();
        }

        let start = self.take(table_size, max_align);

        for &(id, value) in fields {
            value.write(self.buf, start + offsets[id as usize] as usize);
        }

        let vtable_size = 4 + 2 * (max_id + 1);
        let vstart = self.take(vtable_size, 2);
        let vtable_size = vtable_size as u16;
        let table_size = table_size as u16;
        self.buf[vstart..vstart + 2].copy_from_slice(&vtable_size.to_le_bytes());
        self.buf[vstart + 2..vstart + 4].copy_from_slice(&table_size.to_le_bytes());
        for (id, offset) in offsets.iter().enumerate().take(max_id + 1) {
            let at = vstart + 4 + id * 2;
            self.buf[at..at + 2].copy_from_slice(&offset.to_le_bytes());
        }

        let soffset = (start - vstart) as i32;
        self.buf[start..start + 4].copy_from_slice(&soffset.to_le_bytes());
        start
    }

    /// Add the root `Message` table with union tag `body_type` and `body`.
    ///
    /// # Panics
    ///
    /// Panics when the buffer is exhausted.
    pub fn message(&mut self, body_type: u8, body: usize) -> usize {
        self.table(&[(0, Value::U8(body_type)), (1, Value::Offset(body))])
    }

    /// Add a vector of offsets to built tables (the `leds`/`tasks` fields of
    /// `Status`); return its position.
    ///
    /// # Panics
    ///
    /// Panics when the buffer is exhausted.
    pub fn offset_vector(&mut self, targets: &[usize]) -> usize {
        let start = self.take(4 + targets.len() * 4, 4);
        self.buf[start..start + 4].copy_from_slice(&(targets.len() as u32).to_le_bytes());
        for (i, &target) in targets.iter().enumerate() {
            let at = start + 4 + i * 4;
            let rel = (target as u64).wrapping_sub(at as u64) as u32;
            self.buf[at..at + 4].copy_from_slice(&rel.to_le_bytes());
        }
        start
    }

    /// Write the root offset and file id, move the bytes to the front of the
    /// buffer and return them.
    ///
    /// Offsets are relative and the start is 8-aligned, so the move keeps
    /// every field aligned.
    ///
    /// # Panics
    ///
    /// Panics when the buffer is exhausted.
    pub fn finish(mut self, root: usize) -> &'a [u8] {
        let start = self.take(8, 8);
        let rel = (root - start) as u32;
        self.buf[start..start + 4].copy_from_slice(&rel.to_le_bytes());
        self.buf[start + 4..start + 8].copy_from_slice(FILE_ID);
        let len = self.buf.len() - start;
        self.buf.copy_within(start.., 0);
        &self.buf[..len]
    }
}

/// Bounds-checked view of one table in a received buffer.
#[derive(Clone, Copy)]
pub struct Table<'a> {
    /// Whole message buffer.
    buf: &'a [u8],
    /// Table position in `buf`.
    pos: usize,
}

impl<'a> Table<'a> {
    /// Return the root table; `None` if `buf` is short, has another file id
    /// or a bad vtable.
    pub fn root(buf: &'a [u8], file_id: &[u8; 4]) -> Option<Self> {
        if buf.len() < 8 || &buf[4..8] != file_id {
            return None;
        }
        let pos = u32::from_le_bytes(buf[0..4].try_into().ok()?) as usize;
        Self::at(buf, pos)
    }

    /// Return the table at `pos`; `None` if its vtable is out of bounds.
    pub fn at(buf: &'a [u8], pos: usize) -> Option<Self> {
        let this = Self { buf, pos };
        this.vtable_pos()?;
        Some(this)
    }

    /// Return the checked vtable position.
    fn vtable_pos(&self) -> Option<usize> {
        let soffset = i32::from_le_bytes(self.buf.get(self.pos..self.pos + 4)?.try_into().ok()?);
        let vpos = (self.pos as i64).checked_sub(soffset as i64)?;
        if vpos < 0 {
            return None;
        }
        let vpos = vpos as usize;
        let vtable_size =
            u16::from_le_bytes(self.buf.get(vpos..vpos + 2)?.try_into().ok()?) as usize;
        if vtable_size < 4 || vpos + vtable_size > self.buf.len() {
            return None;
        }
        Some(vpos)
    }

    /// Return the position of field `id`; `None` if absent or out of bounds.
    fn field(&self, id: u8) -> Option<usize> {
        let vpos = self.vtable_pos()?;
        let vtable_size =
            u16::from_le_bytes(self.buf.get(vpos..vpos + 2)?.try_into().ok()?) as usize;
        let entry = vpos + 4 + id as usize * 2;
        if entry + 2 > vpos + vtable_size {
            return None;
        }
        let offset = u16::from_le_bytes(self.buf.get(entry..entry + 2)?.try_into().ok()?) as usize;
        if offset == 0 {
            return None;
        }
        let pos = self.pos.checked_add(offset)?;
        if pos >= self.buf.len() {
            return None;
        }
        Some(pos)
    }

    /// Read `ubyte` field `id`.
    pub fn u8(&self, id: u8) -> Option<u8> {
        self.field(id).map(|p| self.buf[p])
    }

    /// Read `uint16` field `id`.
    pub fn u16(&self, id: u8) -> Option<u16> {
        let p = self.field(id)?;
        Some(u16::from_le_bytes(self.buf.get(p..p + 2)?.try_into().ok()?))
    }

    /// Read `uint32` field `id`.
    pub fn u32(&self, id: u8) -> Option<u32> {
        let p = self.field(id)?;
        Some(u32::from_le_bytes(self.buf.get(p..p + 4)?.try_into().ok()?))
    }

    /// Read `ulong` field `id`.
    pub fn u64(&self, id: u8) -> Option<u64> {
        let p = self.field(id)?;
        Some(u64::from_le_bytes(self.buf.get(p..p + 8)?.try_into().ok()?))
    }

    /// Follow table field `id`.
    pub fn table(&self, id: u8) -> Option<Self> {
        let p = self.field(id)?;
        let rel = u32::from_le_bytes(self.buf.get(p..p + 4)?.try_into().ok()?) as usize;
        Self::at(self.buf, p.checked_add(rel)?)
    }

    /// Read string field `id`; `None` if absent, out of bounds or not UTF-8.
    pub fn str(&self, id: u8) -> Option<&'a str> {
        let p = self.field(id)?;
        let rel = u32::from_le_bytes(self.buf.get(p..p + 4)?.try_into().ok()?) as usize;
        let start = p.checked_add(rel)?;
        let len = u32::from_le_bytes(self.buf.get(start..start + 4)?.try_into().ok()?) as usize;
        let data = self.buf.get(start + 4..start + 4 + len)?;
        core::str::from_utf8(data).ok()
    }
}
