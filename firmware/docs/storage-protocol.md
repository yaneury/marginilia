# Marginilia Storage Protocol

This document is the contract between the device firmware and the companion app. Any change to the binary layout must be reflected in both.

---

## Flash Region

The quote bank occupies a dedicated region of the 4 MB SPI NOR flash starting at byte offset **`0x300000`**. All addresses in this document are relative to that base unless otherwise noted.

Sector size: **4096 bytes**. Erase granularity: 1 sector. Write alignment: 4 bytes.

---

## Layout Overview

```
Sector 0 .. (LOOKUP_SECTORS - 1)  →  Lookup table (header + entries)
Sector LOOKUP_SECTORS ..           →  Data records (packed)
```

`LOOKUP_SECTORS` is a compile-time constant in the firmware (currently **1**). Bumping it to 2 doubles the quote capacity; the companion app must be updated to match.

| LOOKUP_SECTORS | Max quotes |
|---|---|
| 1 | 510 |
| 2 | 1022 |
| 3 | 1534 |

*(Capacity reduced by 2 entries per sector-set relative to the no-header layout, because the first 16 bytes of sector 0 are reserved for the header.)*

---

## Lookup Table

The lookup table starts at the base of the flash region (offset 0) and spans `LOOKUP_SECTORS × 4096` bytes. The first 16 bytes of sector 0 are the **header**; lookup entries begin at byte 16.

### Header Format (bytes 0–15 of sector 0)

| Offset | Size | Type     | Field        | Description |
|--------|------|----------|--------------|-------------|
| 0      | 1    | u8       | version      | Format version. Currently **1**. |
| 1      | 1    | u8       | _reserved    | Must be 0. |
| 2      | 2    | u16 LE   | num_entries  | Number of valid lookup entries. |
| 4      | 2    | u16 LE   | db_size      | Number of 4 KB data sectors currently in use. |
| 6      | 2    | u16      | _reserved    | Must be 0. |
| 8      | 4    | u32 LE   | timestamp    | Unix timestamp (seconds) of the last `write` call. |
| 12     | 4    | u32      | _reserved    | Must be 0. |

The companion app must write a valid header whenever it sends a full quote payload. The firmware updates these fields atomically (read-modify-write of sector 0) after writing all data records.

### Entry Format

Each entry is **8 bytes** (two 32-bit little-endian words). Entries start at byte **16** within sector 0.

**Empty sentinel**: all 8 bytes equal `0x00`. On a freshly formatted device all entries are zero. The `num_entries` header field is the authoritative count; the sentinel exists for forward compatibility only.

#### Word 1 — location (bytes 0–3, LE u32)

```
Bit 31 ──────── 22  21 ────────── 10  9 ─────── 0
[  sector : 10 bits  ][  offset : 12 bits  ][ padding ]
```

| Field   | Bits  | Width | Description |
|---------|-------|-------|-------------|
| sector  | 31–22 | 10    | Absolute sector index within the storage region |
| offset  | 21–10 | 12    | Byte offset within that sector |
| padding |  9–0  | 10    | Always 0 |

#### Word 2 — lengths (bytes 4–7, LE u32)

```
Bit 31 ──── 20  19 ──── 14  13 ──── 8  7 ───── 0
[ body_len : 12 ][ author_len : 6 ][ work_len : 6 ][ pad ]
```

| Field      | Bits  | Width | Max value | Description |
|------------|-------|-------|-----------|-------------|
| body_len   | 31–20 | 12    | 4095      | Byte length of the quote body |
| author_len | 19–14 | 6     | 63        | Byte length of the author string |
| work_len   | 13–8  | 6     | 63        | Byte length of the work/title string |
| padding    |  7–0  | 8     | —         | Always 0 |

#### Encode / Decode

```
encode:
  w1 = (sector << 22) | (offset << 10)
  w2 = (body_len << 20) | (author_len << 14) | (work_len << 8)
  bytes[0..4] = w1 as LE u32
  bytes[4..8] = w2 as LE u32

decode:
  w1 = LE u32 from bytes[0..4]
  w2 = LE u32 from bytes[4..8]
  sector     = (w1 >> 22) & 0x3FF
  offset     = (w1 >> 10) & 0xFFF
  body_len   = (w2 >> 20) & 0xFFF
  author_len = (w2 >> 14) & 0x3F
  work_len   = (w2 >>  8) & 0x3F
```

---

## Data Records

Data records are packed sequentially in sectors starting at sector `LOOKUP_SECTORS`.

### Record Layout

```
[ body bytes ][ author bytes ][ work bytes ][ 0–3 padding bytes ]
```

- No delimiters. Field lengths come from the lookup table entry.
- The record is padded to the next **4-byte boundary**. Padding bytes are `0x00`.
- When a record would not fit in the remaining space of the current sector, it begins at offset 0 of the next sector. That sector is erased before the first write.

### Field Encoding

All strings are **UTF-8**, no null terminator.

---

## Byte Order

All multi-byte integer values are **little-endian**. The ESP32 is natively little-endian so no byte swapping is required on the device side.

---

## Write Flow

The companion app sends a complete quote payload on every BLE sync. The firmware:

1. Erases all lookup sectors and writes zeros (format).
2. Writes data records sequentially, one per quote.
3. Writes lookup entries.
4. Updates the header: `num_entries`, `db_size`, `timestamp`, and `version = 1`.

Steps 1–4 happen atomically from the app's perspective. The header is the last thing written; a device that lost power mid-write will have `version = 0` (erased sector reads as `0xFF`) and the firmware will treat the store as uninitialized.

---

## Initialization

On first use (or after a wipe), the firmware erases all lookup sectors and writes `0x00` over them. The data region is erased on demand (one sector at a time, when first written). The header `version` byte is written in the same pass; after `format()` it is set to `CURRENT_VERSION` with all other header fields at 0.
