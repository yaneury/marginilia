# marginilia

DIY e-ink display that rotates saved quotes. An ESP32 display detects your phone via BLE, checks if its quotes are stale, and refreshes from a backend API.

## Repo structure

```
marginilia/
├── server/     # Rust/Axum backend — serves quotes over HTTP
├── app/        # React Native iOS app — BLE bridge between server and device
└── device/     # ESP32 embedded code (e-ink display) — not yet started
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

## App (`app/`)

React Native (0.76.5) iOS app. Acts as a BLE bridge: fetches quotes from the server, then writes them to the ESP32 over BLE.

- `src/api.ts` — backend fetch + API key stored in iOS Keychain (`setApiKey()` must be called once before first use)
- `src/ble.ts` — BLE scan/connect/chunked write to the display
- `src/constants.ts` — BLE UUIDs, server base URL, storage keys

### Setup

```bash
cd app
npm install
cd ios && pod install && cd ..
```

iOS BLE permissions (`ios/marginilia/Info.plist`):
```xml
<key>NSBluetoothAlwaysUsageDescription</key>
<string>Marginilia uses Bluetooth to sync quotes to your e-ink display.</string>
<key>NSBluetoothPeripheralUsageDescription</key>
<string>Marginilia uses Bluetooth to sync quotes to your e-ink display.</string>
```

Set the API key once (e.g. from a dev button), and set `BASE_URL` in `src/constants.ts` to the deployed server URL.

Run: `npx react-native run-ios`

### BLE protocol

The app expects the ESP32 to:
- Advertise the name `Marginilia`
- Expose service UUID `4a1b2c3d-0001-0000-0000-000000000000`
- Expose a writable characteristic `4a1b2c3d-0002-0000-0000-000000000000`

Payload (UTF-8, chunked into 20-byte BLE writes):
```
CHECKSUM:<sha256>\n
[{"body":"...","author":"...","work":"..."},...]
\nEND\n
```

The device reassembles, stores, verifies, and refreshes the carousel.

## Device (`device/`)

ESP32 firmware for the e-ink display. Not yet implemented in this repo.
