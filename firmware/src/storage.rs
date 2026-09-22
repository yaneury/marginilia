// FLASH LAYOUT (base = FLASH_OFFSET)
//
// Sector 0 .. LOOKUP_SECTORS-1 : Lookup table (header + entries)
// Sector LOOKUP_SECTORS ..     : Data records (packed)
//
// LOOKUP TABLE HEADER (first 16 bytes of sector 0)
//
// Byte  0      version       u8      currently 1
// Byte  1      _reserved     u8      always 0
// Bytes 2-3    num_entries   u16 LE  count of valid lookup entries
// Bytes 4-5    db_size       u16 LE  number of 4 KB data sectors in use
// Bytes 6-7    _reserved     u16     always 0
// Bytes 8-11   timestamp     u32 LE  unix time of last append (caller-provided)
// Bytes 12-15  _reserved     u32     always 0
//
// LOOKUP ENTRIES (starting at byte 16)
//
// 8 bytes per entry, little-endian. Unwritten positions are 0xFF (erased state).
// Valid entries are at indices 0 .. num_entries-1 per the header; positions beyond
// that are 0xFF and must not be read.
//
// Word 1 (bytes 0-3, LE u32):
//   bits 31-22  sector    (10 bits)  absolute sector index in storage region
//   bits 21-10  offset    (12 bits)  byte offset within that sector
//   bits  9-0   padding   (10 bits)  always 0
//
// Word 2 (bytes 4-7, LE u32):
//   bits 31-20  body_len  (12 bits)  byte length of quote body
//   bits 19-14  author_len (6 bits)  byte length of author field
//   bits 13-8   work_len   (6 bits)  byte length of work/title field
//   bits  7-0   padding    (8 bits)  always 0
//
// Encode:  w1 = (sector << 22) | (offset << 10)
//          w2 = (body_len << 20) | (author_len << 14) | (work_len << 8)
// Decode:  sector     = (w1 >> 22) & 0x3FF
//          offset     = (w1 >> 10) & 0xFFF
//          body_len   = (w2 >> 20) & 0xFFF
//          author_len = (w2 >> 14) & 0x3F
//          work_len   = (w2 >>  8) & 0x3F
//
// Capacity: (LOOKUP_SECTORS * 4096 - 16) / 8 entries
//   LOOKUP_SECTORS = 1 → 510 entries
//   LOOKUP_SECTORS = 2 → 1022 entries
//
// DATA RECORDS
//
// Packed sequentially in sectors >= DATA_START_SECTOR.
// Format: [ body bytes ][ author bytes ][ work bytes ][ 0-3 padding bytes ]
// Padding rounds total record length to next 4-byte boundary.
// No delimiters — lengths come from the lookup entry.
// When a record doesn't fit in the remaining sector space, it starts at
// offset 0 of the next sector (that sector is erased first).
//
// BYTE ORDER: little-endian throughout (ESP32 native; no conversion needed).
//
// See docs/storage-protocol.md for the companion app contract.

use crate::model::Quote;
use embedded_storage::nor_flash::NorFlash;

const FLASH_OFFSET: u32 = 0x300_000;
const SECTOR_SIZE: u32 = 4_096;
const HEADER_SIZE: u32 = 16;
const CURRENT_VERSION: u8 = 1;
pub const LOOKUP_SECTORS: u32 = 1;
const MAX_ENTRIES: u32 = (LOOKUP_SECTORS * SECTOR_SIZE - HEADER_SIZE) / 8;
const DATA_START_SECTOR: u32 = LOOKUP_SECTORS;

#[derive(Debug)]
pub enum StorageError {
    Flash,
    Full,
    OutOfBounds,
    Corrupt,
    BufTooSmall,
}

struct LookupHeader {
    version: u8,
    num_entries: u16,
    db_size: u16,
    timestamp: u32,
}

impl LookupHeader {
    fn encode(&self) -> [u8; HEADER_SIZE as usize] {
        let mut out = [0u8; HEADER_SIZE as usize];
        out[0] = self.version;
        out[2..4].copy_from_slice(&self.num_entries.to_le_bytes());
        out[4..6].copy_from_slice(&self.db_size.to_le_bytes());
        out[8..12].copy_from_slice(&self.timestamp.to_le_bytes());
        out
    }

    fn decode(bytes: &[u8; HEADER_SIZE as usize]) -> Self {
        Self {
            version: bytes[0],
            num_entries: u16::from_le_bytes([bytes[2], bytes[3]]),
            db_size: u16::from_le_bytes([bytes[4], bytes[5]]),
            timestamp: u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]),
        }
    }
}

struct LookupEntry {
    sector: u32,
    offset: u32,
    body_len: u32,
    author_len: u32,
    work_len: u32,
}

impl LookupEntry {
    fn padded_record_len(&self) -> u32 {
        let total = self.body_len + self.author_len + self.work_len;
        (total + 3) & !3
    }

    fn encode(&self) -> [u8; 8] {
        let w1 = (self.sector << 22) | (self.offset << 10);
        let w2 = (self.body_len << 20) | (self.author_len << 14) | (self.work_len << 8);
        let mut out = [0u8; 8];
        out[0..4].copy_from_slice(&w1.to_le_bytes());
        out[4..8].copy_from_slice(&w2.to_le_bytes());
        out
    }

    fn decode(bytes: &[u8; 8]) -> Self {
        let w1 = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let w2 = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        Self {
            sector: (w1 >> 22) & 0x3FF,
            offset: (w1 >> 10) & 0xFFF,
            body_len: (w2 >> 20) & 0xFFF,
            author_len: (w2 >> 14) & 0x3F,
            work_len: (w2 >> 8) & 0x3F,
        }
    }
}

pub struct QuoteStore<F> {
    flash: F,
    merge_buf: [u8; SECTOR_SIZE as usize],
}

impl<F: NorFlash> QuoteStore<F> {
    pub fn new(flash: F) -> Self {
        Self { flash, merge_buf: [0u8; SECTOR_SIZE as usize] }
    }

    /// Erase all lookup sectors and write the initial header. Call once on first use or factory reset.
    pub fn format(&mut self) -> Result<(), StorageError> {
        for s in 0..LOOKUP_SECTORS {
            let addr = FLASH_OFFSET + s * SECTOR_SIZE;
            self.flash.erase(addr, addr + SECTOR_SIZE).map_err(|_| StorageError::Flash)?;
        }
        let header = LookupHeader { version: CURRENT_VERSION, num_entries: 0, db_size: 0, timestamp: 0 };
        self.flash.write(FLASH_OFFSET, &header.encode()).map_err(|_| StorageError::Flash)?;
        Ok(())
    }

    /// Return the number of stored quotes (O(1), reads from header).
    pub fn count(&mut self) -> Result<u32, StorageError> {
        let header = self.read_header()?;
        if header.version != CURRENT_VERSION {
            return Ok(0);
        }
        Ok(header.num_entries as u32)
    }

    /// Read the quote at `index` into `buf`. `buf` must be >= body_len + author_len + work_len bytes.
    pub fn fetch<'a>(&mut self, index: u32, buf: &'a mut [u8]) -> Result<Quote<'a>, StorageError> {
        if index >= MAX_ENTRIES {
            return Err(StorageError::OutOfBounds);
        }
        let header = self.read_header()?;
        if header.version != CURRENT_VERSION {
            return Err(StorageError::Corrupt);
        }
        if index >= header.num_entries as u32 {
            return Err(StorageError::OutOfBounds);
        }
        let entry = self.read_lookup(index)?;
        let total = (entry.body_len + entry.author_len + entry.work_len) as usize;
        if buf.len() < total {
            return Err(StorageError::BufTooSmall);
        }
        let abs_addr = FLASH_OFFSET + entry.sector * SECTOR_SIZE + entry.offset;
        self.flash.read(abs_addr, &mut buf[..total]).map_err(|_| StorageError::Flash)?;
        let body_end = entry.body_len as usize;
        let author_end = body_end + entry.author_len as usize;
        let work_end = author_end + entry.work_len as usize;
        let buf: &'a [u8] = buf;
        let body = core::str::from_utf8(&buf[..body_end]).map_err(|_| StorageError::Corrupt)?;
        let author = core::str::from_utf8(&buf[body_end..author_end]).map_err(|_| StorageError::Corrupt)?;
        let work = core::str::from_utf8(&buf[author_end..work_end]).map_err(|_| StorageError::Corrupt)?;
        Ok(Quote { body, author, work })
    }

    /// Append quotes to the store and update the header. `timestamp` is the unix time of this call.
    ///
    /// Sectors are managed internally. If a quote doesn't fit in the current data sector it is
    /// written at offset 0 of the next sector (which is erased first). The lookup entries and
    /// header are updated atomically after all data is written.
    pub fn append(&mut self, quotes: &[Quote<'_>], timestamp: u32) -> Result<(), StorageError> {
        if quotes.is_empty() {
            return Ok(());
        }

        let header = self.read_header()?;
        if header.version != CURRENT_VERSION {
            return Err(StorageError::Corrupt);
        }

        let n = header.num_entries as u32;
        if n + quotes.len() as u32 > MAX_ENTRIES {
            return Err(StorageError::Full);
        }

        // Determine the starting write position in the data region.
        let (mut cur_sector, mut cur_offset) = if n == 0 {
            let sector = DATA_START_SECTOR;
            self.flash
                .erase(FLASH_OFFSET + sector * SECTOR_SIZE, FLASH_OFFSET + (sector + 1) * SECTOR_SIZE)
                .map_err(|_| StorageError::Flash)?;
            (sector, 0u32)
        } else {
            let last = self.read_lookup(n - 1)?;
            (last.sector, last.offset + last.padded_record_len())
        };

        for (i, quote) in quotes.iter().enumerate() {
            let idx = n + i as u32;
            let body_len = quote.body.len() as u32;
            let author_len = quote.author.len() as u32;
            let work_len = quote.work.len() as u32;

            if body_len > 0xFFF || author_len > 0x3F || work_len > 0x3F {
                return Err(StorageError::Full);
            }

            let padded_len = (body_len + author_len + work_len + 3) & !3;

            if cur_offset + padded_len > SECTOR_SIZE {
                cur_sector += 1;
                cur_offset = 0;
                self.flash
                    .erase(
                        FLASH_OFFSET + cur_sector * SECTOR_SIZE,
                        FLASH_OFFSET + (cur_sector + 1) * SECTOR_SIZE,
                    )
                    .map_err(|_| StorageError::Flash)?;
            }

            // Pack record into merge_buf and write to data sector.
            self.merge_buf[..padded_len as usize].fill(0);
            let mut pos = 0usize;
            self.merge_buf[pos..pos + quote.body.len()].copy_from_slice(quote.body.as_bytes());
            pos += quote.body.len();
            self.merge_buf[pos..pos + quote.author.len()].copy_from_slice(quote.author.as_bytes());
            pos += quote.author.len();
            self.merge_buf[pos..pos + quote.work.len()].copy_from_slice(quote.work.as_bytes());

            let abs_data = FLASH_OFFSET + cur_sector * SECTOR_SIZE + cur_offset;
            {
                let (flash, buf) = (&mut self.flash, &mut self.merge_buf);
                flash.write(abs_data, &buf[..padded_len as usize]).map_err(|_| StorageError::Flash)?;
            }

            // Write lookup entry directly into erased (0xFF) position — no RMW needed.
            let entry = LookupEntry { sector: cur_sector, offset: cur_offset, body_len, author_len, work_len };
            let abs_entry = FLASH_OFFSET + HEADER_SIZE + idx * 8;
            self.flash.write(abs_entry, &entry.encode()).map_err(|_| StorageError::Flash)?;

            cur_offset += padded_len;
        }

        let new_count = (n + quotes.len() as u32) as u16;
        let db_size = (cur_sector - DATA_START_SECTOR + 1) as u16;
        self.update_header(new_count, db_size, timestamp)
    }

    fn read_header(&mut self) -> Result<LookupHeader, StorageError> {
        let mut buf = [0u8; HEADER_SIZE as usize];
        self.flash.read(FLASH_OFFSET, &mut buf).map_err(|_| StorageError::Flash)?;
        Ok(LookupHeader::decode(&buf))
    }

    // RMW sector 0 to update the header while preserving the lookup entries it contains.
    fn update_header(&mut self, num_entries: u16, db_size: u16, timestamp: u32) -> Result<(), StorageError> {
        let abs = FLASH_OFFSET;
        {
            let (flash, buf) = (&mut self.flash, &mut self.merge_buf);
            flash.read(abs, buf).map_err(|_| StorageError::Flash)?;
        }
        let header = LookupHeader { version: CURRENT_VERSION, num_entries, db_size, timestamp };
        self.merge_buf[..HEADER_SIZE as usize].copy_from_slice(&header.encode());
        {
            let (flash, buf) = (&mut self.flash, &mut self.merge_buf);
            flash.erase(abs, abs + SECTOR_SIZE).map_err(|_| StorageError::Flash)?;
            flash.write(abs, buf).map_err(|_| StorageError::Flash)?;
        }
        Ok(())
    }

    fn read_lookup(&mut self, index: u32) -> Result<LookupEntry, StorageError> {
        let abs_addr = FLASH_OFFSET + HEADER_SIZE + index * 8;
        let mut buf = [0u8; 8];
        self.flash.read(abs_addr, &mut buf).map_err(|_| StorageError::Flash)?;
        Ok(LookupEntry::decode(&buf))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;
    use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash};

    struct MockFlash {
        data: Vec<u8>,
    }

    impl MockFlash {
        fn new(sectors: u32) -> Self {
            Self { data: vec![0xFF; (sectors * SECTOR_SIZE) as usize] }
        }

        fn idx(&self, abs_addr: u32) -> usize {
            (abs_addr - FLASH_OFFSET) as usize
        }
    }

    impl ErrorType for MockFlash {
        type Error = Infallible;
    }

    impl ReadNorFlash for MockFlash {
        const READ_SIZE: usize = 1;

        fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Infallible> {
            let i = self.idx(offset);
            bytes.copy_from_slice(&self.data[i..i + bytes.len()]);
            Ok(())
        }

        fn capacity(&self) -> usize {
            self.data.len()
        }
    }

    impl NorFlash for MockFlash {
        const WRITE_SIZE: usize = 4;
        const ERASE_SIZE: usize = SECTOR_SIZE as usize;

        fn erase(&mut self, from: u32, to: u32) -> Result<(), Infallible> {
            let start = self.idx(from);
            let end = self.idx(to);
            self.data[start..end].fill(0xFF);
            Ok(())
        }

        fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Infallible> {
            let i = self.idx(offset);
            for (j, &b) in bytes.iter().enumerate() {
                self.data[i + j] &= b;
            }
            Ok(())
        }
    }

    fn make_store(sectors: u32) -> QuoteStore<MockFlash> {
        QuoteStore::new(MockFlash::new(sectors))
    }

    #[test]
    fn test_lookup_entry_roundtrip() {
        let entry = LookupEntry { sector: 5, offset: 128, body_len: 200, author_len: 12, work_len: 30 };
        let decoded = LookupEntry::decode(&entry.encode());
        assert_eq!(decoded.sector, 5);
        assert_eq!(decoded.offset, 128);
        assert_eq!(decoded.body_len, 200);
        assert_eq!(decoded.author_len, 12);
        assert_eq!(decoded.work_len, 30);
    }

    #[test]
    fn test_header_roundtrip() {
        let header = LookupHeader { version: 1, num_entries: 42, db_size: 3, timestamp: 0xDEAD_BEEF };
        let decoded = LookupHeader::decode(&header.encode());
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.num_entries, 42);
        assert_eq!(decoded.db_size, 3);
        assert_eq!(decoded.timestamp, 0xDEAD_BEEF);
    }

    #[test]
    fn test_count_after_format() {
        let mut store = make_store(10);
        store.format().unwrap();
        assert_eq!(store.count().unwrap(), 0);
    }

    #[test]
    fn test_append_and_count() {
        let mut store = make_store(10);
        store.format().unwrap();
        let quotes = [
            Quote { body: "Hello", author: "Alice", work: "Book A" },
            Quote { body: "World", author: "Bob", work: "Book B" },
            Quote { body: "Rust!", author: "Carol", work: "Book C" },
        ];
        store.append(&quotes, 0).unwrap();
        assert_eq!(store.count().unwrap(), 3);
    }

    #[test]
    fn test_header_fields_after_append() {
        let mut store = make_store(10);
        store.format().unwrap();
        let quotes = [
            Quote { body: "A", author: "B", work: "C" },
            Quote { body: "D", author: "E", work: "F" },
        ];
        store.append(&quotes, 0x1234_5678).unwrap();

        let header = store.read_header().unwrap();
        assert_eq!(header.version, CURRENT_VERSION);
        assert_eq!(header.num_entries, 2);
        assert_eq!(header.db_size, 1);
        assert_eq!(header.timestamp, 0x1234_5678);
    }

    #[test]
    fn test_fetch_single() {
        let mut store = make_store(10);
        store.format().unwrap();
        store.append(&[Quote { body: "Be yourself", author: "Wilde", work: "De Profundis" }], 0).unwrap();

        let mut buf = [0u8; 64];
        let q = store.fetch(0, &mut buf).unwrap();
        assert_eq!(q.body, "Be yourself");
        assert_eq!(q.author, "Wilde");
        assert_eq!(q.work, "De Profundis");
    }

    #[test]
    fn test_fetch_multiple() {
        let mut store = make_store(10);
        store.format().unwrap();
        store
            .append(
                &[
                    Quote { body: "First", author: "A", work: "X" },
                    Quote { body: "Second", author: "B", work: "Y" },
                    Quote { body: "Third", author: "C", work: "Z" },
                ],
                0,
            )
            .unwrap();

        let mut buf0 = [0u8; 32];
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        let q0 = store.fetch(0, &mut buf0).unwrap();
        let q1 = store.fetch(1, &mut buf1).unwrap();
        let q2 = store.fetch(2, &mut buf2).unwrap();

        assert_eq!((q0.body, q0.author, q0.work), ("First", "A", "X"));
        assert_eq!((q1.body, q1.author, q1.work), ("Second", "B", "Y"));
        assert_eq!((q2.body, q2.author, q2.work), ("Third", "C", "Z"));
    }

    #[test]
    fn test_append_multiple_batches() {
        let mut store = make_store(10);
        store.format().unwrap();
        store
            .append(
                &[
                    Quote { body: "First", author: "A", work: "X" },
                    Quote { body: "Second", author: "B", work: "Y" },
                ],
                1,
            )
            .unwrap();
        store.append(&[Quote { body: "Third", author: "C", work: "Z" }], 2).unwrap();

        assert_eq!(store.count().unwrap(), 3);

        let mut buf0 = [0u8; 32];
        let mut buf1 = [0u8; 32];
        let mut buf2 = [0u8; 32];
        let q0 = store.fetch(0, &mut buf0).unwrap();
        let q1 = store.fetch(1, &mut buf1).unwrap();
        let q2 = store.fetch(2, &mut buf2).unwrap();

        assert_eq!((q0.body, q0.author, q0.work), ("First", "A", "X"));
        assert_eq!((q1.body, q1.author, q1.work), ("Second", "B", "Y"));
        assert_eq!((q2.body, q2.author, q2.work), ("Third", "C", "Z"));
    }

    #[test]
    fn test_out_of_bounds() {
        let mut store = make_store(10);
        store.format().unwrap();
        store.append(&[Quote { body: "One", author: "A", work: "B" }], 0).unwrap();

        let mut buf = [0u8; 32];
        assert!(matches!(store.fetch(1, &mut buf), Err(StorageError::OutOfBounds)));
        assert!(matches!(store.fetch(100, &mut buf), Err(StorageError::OutOfBounds)));
    }

    #[test]
    fn test_buf_too_small() {
        let mut store = make_store(10);
        store.format().unwrap();
        store.append(&[Quote { body: "Hello world", author: "Author", work: "Work" }], 0).unwrap();
        let mut buf = [0u8; 4];
        assert!(matches!(store.fetch(0, &mut buf), Err(StorageError::BufTooSmall)));
    }

    #[test]
    fn test_sector_crossing() {
        let mut store = make_store(20);
        store.format().unwrap();

        // body_len=3900, author=1, work=1 → padded to 3904; two won't fit in one 4096-byte sector
        let body = "x".repeat(3900);
        store
            .append(
                &[
                    Quote { body: &body, author: "A", work: "W" },
                    Quote { body: &body, author: "B", work: "W" },
                ],
                0,
            )
            .unwrap();

        assert_eq!(store.count().unwrap(), 2);
        assert_eq!(store.read_header().unwrap().db_size, 2);

        let mut buf = vec![0u8; 4096];

        let q = store.fetch(0, &mut buf).unwrap();
        assert_eq!(q.body, body.as_str());
        assert_eq!(q.author, "A");

        let q = store.fetch(1, &mut buf).unwrap();
        assert_eq!(q.body, body.as_str());
        assert_eq!(q.author, "B");
    }

    #[test]
    fn test_sector_crossing_across_batches() {
        let mut store = make_store(20);
        store.format().unwrap();

        let body = "x".repeat(3900);
        store.append(&[Quote { body: &body, author: "A", work: "W" }], 1).unwrap();
        store.append(&[Quote { body: &body, author: "B", work: "W" }], 2).unwrap();

        assert_eq!(store.count().unwrap(), 2);
        assert_eq!(store.read_header().unwrap().db_size, 2);
        assert_eq!(store.read_header().unwrap().timestamp, 2);

        let mut buf = vec![0u8; 4096];
        let q = store.fetch(0, &mut buf).unwrap();
        assert_eq!(q.author, "A");
        let q = store.fetch(1, &mut buf).unwrap();
        assert_eq!(q.author, "B");
    }
}
