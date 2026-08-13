# app — marginilia iOS

React Native app. Acts as a BLE bridge between the backend and the e-ink display.

## Setup

### 1. Initialize the React Native project

On your Mac (requires Node, Watchman, Xcode):

```bash
cd marginilia/app
npx react-native@0.76.5 init marginilia --template react-native-template-typescript --skip-install
# Then copy the src/ files from this directory into the generated project
```

Or if you prefer to start from scratch:

```bash
npx @react-native-community/cli@latest init marginilia
```

### 2. Install dependencies

```bash
npm install
cd ios && pod install && cd ..
```

### 3. iOS permissions (required for BLE)

In `ios/marginilia/Info.plist`, add:

```xml
<key>NSBluetoothAlwaysUsageDescription</key>
<string>Marginilia uses Bluetooth to sync quotes to your e-ink display.</string>
<key>NSBluetoothPeripheralUsageDescription</key>
<string>Marginilia uses Bluetooth to sync quotes to your e-ink display.</string>
```

### 4. Set your API key (first launch)

The app reads the API key from the iOS Keychain. To set it, call `setApiKey()` from `api.ts` once:

```ts
import { setApiKey } from './src/api';
await setApiKey('your-secret-key-here');
```

You can wire this to a settings screen or a one-time prompt. For now, call it from a dev button.

### 5. Set the backend URL

Edit `src/constants.ts` and replace:

```ts
BASE_URL: 'https://your-server-url-here.com',
```

with your deployed server URL.

### 6. Run

```bash
npx react-native run-ios
```

## BLE Protocol

The app expects the ESP32 to:
- Advertise the name `Marginilia`
- Expose service UUID `4a1b2c3d-0001-0000-0000-000000000000`
- Expose a writable characteristic `4a1b2c3d-0002-0000-0000-000000000000`

Payload format (UTF-8, chunked into 20-byte BLE writes):

```
CHECKSUM:<sha256>\n
[{"body":"...","author":"...","work":"..."},...]
\nEND\n
```

The device reassembles, stores, verifies, and refreshes the carousel.

## File Structure

```
app/
├── src/
│   ├── App.tsx        # Main UI (sync button, BLE status, last sync time)
│   ├── api.ts         # Backend fetch + Keychain API key
│   ├── ble.ts         # BLE scan, connect, chunk-write
│   └── constants.ts   # UUIDs, URLs, storage keys
├── index.js           # RN entry point
├── package.json
└── tsconfig.json
```
