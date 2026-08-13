// BLE service and characteristic UUIDs.
// The ESP32 must advertise the SERVICE_UUID and expose QUOTES_CHAR_UUID
// as a writable characteristic.
//
// Copy these into your embedded code.

export const BLE = {
  // Name the ESP32 advertises — used for scanning
  DEVICE_NAME: 'Marginilia',

  // Custom GATT service
  SERVICE_UUID: '4a1b2c3d-0001-0000-0000-000000000000',

  // Writable characteristic — app writes chunked quote payload here
  QUOTES_CHAR_UUID: '4a1b2c3d-0002-0000-0000-000000000000',

  // Max bytes per BLE write (conservative; ESP32 default ATT MTU - 3)
  CHUNK_SIZE: 20,

  // Scan timeout in ms
  SCAN_TIMEOUT_MS: 10_000,
};

// Backend
export const API = {
  BASE_URL: 'https://your-server-url-here.com', // replace before deploying
  QUOTES_PATH: '/quotes',
};

// AsyncStorage keys
export const STORAGE = {
  LAST_SYNC: 'last_sync_ts',
  LAST_CHECKSUM: 'last_checksum',
};
