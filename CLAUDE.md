# marginilia

DIY e-ink display that rotates saved quotes. An ESP32 display checks if its quotes are stale and refreshes from a backend API via a companion iOS app acting as a BLE bridge.

## Repo structure

```
marginilia/
├── server/     # Rust/Axum backend — serves quotes over HTTP
├── ios/        # Swift/SwiftUI iOS app — BLE bridge between server and device
└── firmware/   # ESP32 embedded code (e-ink display, no_std Rust)
```

## Server (`server/`)

Rust + Axum. Reads quotes from a YAML file into memory at startup.

### Running locally

```bash
cd server
API_KEY=your-secret-key cargo run
```

Reads `quotes.yaml` from the current directory by default; override with `QUOTES_PATH`.

### Environment variables

| Variable      | Default        | Description                                |
|---------------|----------------|--------------------------------------------|
| `API_KEY`     | (required)     | Pre-shared key sent in `x-api-key` header   |
| `QUOTES_PATH` | `quotes.yaml`  | Path to the quotes YAML file                |
| `PORT`        | `3000`         | Port to listen on                           |

### API

#### `GET /quotes?since=<unix_ms>`

Returns quotes newer than `since` (or all quotes if `since` is omitted/`0`), plus the latest `created_at` timestamp across all quotes. This is a timestamp-based sync (see `server/src/quotes.rs`), replacing an earlier checksum-based design.

**Headers:** `x-api-key: <your-key>`

**Response:**
```json
{
  "latest_timestamp": 1786901066295,
  "quotes": [
    {
      "body": "Quote text here.",
      "author": "Author Name",
      "work": "Book or Article Title",
      "created_at": 1786901066295
    }
  ]
}
```

### Quote format (`quotes.yaml`)

```yaml
- body: "The quote text."
  author: "Author Name"
  work: "Source title (book, poem, article)"
  created_at: 1786901066295
```

`work` can be an empty string if the source is unknown. Multiline quote bodies use YAML literal blocks (`|`).

## iOS App (`ios/`)

SwiftUI iOS app. Acts as a BLE bridge: fetches quotes from the server, then writes them to the ESP32 over BLE. Currently wires up backend quote fetching and shows connection status; BLE sync to the device is not yet implemented.

Key files:
- `ios/marginilia/ContentView.swift` — main UI, `QuoteService` actor for backend calls

The app hardcodes `baseURL = "http://workstation:3000"` and `apiKey = "your-secret-key"`. Update these before running against a real server.

Run from Xcode or `xcodebuild`.

## Firmware (`firmware/`)

`no_std` Rust for ESP32. Drives a Waveshare 7.5" V2 (UC8179) e-ink display. Rotates quotes in a Fisher-Yates shuffled carousel, updating the display every 3 minutes.

### Toolchain & build

Requires the `esp` Rust toolchain channel (Xtensa fork):

```bash
cd firmware
cargo build          # debug
cargo build --release
```

Flash and monitor:

```bash
cargo run            # runs: espflash flash --monitor --chip esp32
```

There is also a `clear` binary that blanks the display:

```bash
cargo run --bin clear
```

### GPIO wiring

| Signal | GPIO |
|--------|------|
| DC     | 27   |
| RST    | 26   |
| CS     | 15   |
| BUSY   | 25   |
| SCK    | 13   |
| MOSI   | 14   |

SPI bus: SPI2, 4 MHz, Mode 0.

### Display color note

The Waveshare 7.5" V2 (UC8179) inverts colors at the hardware level. `Color::White` (1-bits) renders as physical black ink; `Color::Black` (0-bits) renders as physical white background. All color values in `src/display.rs` are intentionally swapped to compensate.

### Quote source

On startup the firmware reads from `QuoteStore` (SPI NOR flash). If the flash is empty (no BLE sync has happened yet), it falls back to the compile-time `QUOTES` slice in `src/sample.rs` so the device is always usable out of the box.

### BLE receive (not yet implemented)

The firmware needs to accept quote payloads written by the iOS app over BLE. This requires `esp-wifi` with the `ble` feature. As of now no stable `esp-wifi` release is compatible with `esp-hal ~1.1.0` — the `__esp_wifi_builtin_scheduler` feature that `esp-wifi ≥0.14` needs does not exist in any published `esp-hal` version. Once a compatible `esp-wifi` is available, the BLE GATT peripheral layer and the JSON payload parser can be added here.

### Flash storage (`src/storage.rs`)

`QuoteStore<F>` manages a quote database in SPI NOR flash. This section is the contract between the firmware and the companion app — any change to the binary layout must be reflected in both.

#### Flash region

The quote bank occupies a dedicated region of the 4 MB SPI NOR flash starting at byte offset **`0x300000`**. Sector size: **4096 bytes**. Write alignment: 4 bytes.

#### Layout overview

```
Sector 0 .. (LOOKUP_SECTORS - 1)  →  Lookup table (header + entries)
Sector LOOKUP_SECTORS ..           →  Data records (packed)
```

`LOOKUP_SECTORS` is a compile-time constant (currently **1**). Bumping it doubles the quote capacity; the companion app must be updated to match.

| LOOKUP_SECTORS | Max quotes |
|---|---|
| 1 | 510 |
| 2 | 1022 |
| 3 | 1534 |

#### Lookup table header (bytes 0–15 of sector 0)

| Offset | Size | Type     | Field        | Description |
|--------|------|----------|--------------|-------------|
| 0      | 1    | u8       | version      | Format version. Currently **1**. |
| 1      | 1    | u8       | _reserved    | Must be 0. |
| 2      | 2    | u16 LE   | num_entries  | Number of valid lookup entries. |
| 4      | 2    | u16 LE   | db_size      | Number of 4 KB data sectors currently in use. |
| 6      | 2    | u16      | _reserved    | Must be 0. |
| 8      | 4    | u32 LE   | timestamp    | Unix timestamp (seconds) of the last write. |
| 12     | 4    | u32      | _reserved    | Must be 0. |

The header `version` byte is written last; a device that lost power mid-write will have `version = 0xFF` (erased) and the firmware treats the store as uninitialized.

#### Lookup entry format (8 bytes each, starting at byte 16)

**Word 1 — location (bytes 0–3, LE u32)**

```
Bit 31 ──────── 22  21 ────────── 10  9 ─────── 0
[  sector : 10 bits  ][  offset : 12 bits  ][ padding ]
```

**Word 2 — lengths (bytes 4–7, LE u32)**

```
Bit 31 ──── 20  19 ──── 14  13 ──── 8  7 ───── 0
[ body_len : 12 ][ author_len : 6 ][ work_len : 6 ][ pad ]
```

| Field      | Bits  | Width | Max value |
|------------|-------|-------|-----------|
| body_len   | 31–20 | 12    | 4095      |
| author_len | 19–14 | 6     | 63        |
| work_len   | 13–8  | 6     | 63        |

Encode/decode:

```
encode:
  w1 = (sector << 22) | (offset << 10)
  w2 = (body_len << 20) | (author_len << 14) | (work_len << 8)

decode:
  sector     = (w1 >> 22) & 0x3FF
  offset     = (w1 >> 10) & 0xFFF
  body_len   = (w2 >> 20) & 0xFFF
  author_len = (w2 >> 14) & 0x3F
  work_len   = (w2 >>  8) & 0x3F
```

#### Data records

Packed sequentially in sectors starting at sector `LOOKUP_SECTORS`.

```
[ body bytes ][ author bytes ][ work bytes ][ 0–3 padding bytes ]
```

- No delimiters — field lengths come from the lookup entry.
- Records are padded to the next **4-byte boundary** (`0x00` fill).
- When a record would not fit in the remaining sector space, it starts at offset 0 of the next sector (erased first).
- All strings are UTF-8, no null terminator.
- All multi-byte integers are little-endian (ESP32 native; no byte swapping needed).

#### Write flow (companion app → firmware)

1. Erase all lookup sectors (format).
2. Write data records sequentially, one per quote.
3. Write lookup entries.
4. Update header: `num_entries`, `db_size`, `timestamp`, `version = 1`.

The header is written last. A device that lost power before step 4 will have `version = 0xFF` and treats the store as uninitialized.
