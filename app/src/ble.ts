// BLE layer: scan, connect, and write the quote payload to the display.
//
// Protocol (simple, text-based):
//   - JSON-encode the QuotesResponse
//   - Prefix with "CHECKSUM:<sha256>\n"
//   - Chunk into BLE.CHUNK_SIZE bytes and write sequentially
//   - Send "END\n" as the final chunk
//
// The ESP32 reassembles, verifies the checksum, stores, and refreshes the carousel.

import { BleManager, Device, State } from 'react-native-ble-plx';
import { BLE } from './constants';
import { QuotesResponse } from './api';

const manager = new BleManager();

export type BleStatus =
  | 'idle'
  | 'scanning'
  | 'connecting'
  | 'connected'
  | 'writing'
  | 'done'
  | 'error';

export interface BleState {
  status: BleStatus;
  device: Device | null;
  error: string | null;
}

// Check if Bluetooth is powered on. Returns true when ready.
export async function waitForBluetooth(): Promise<boolean> {
  return new Promise(resolve => {
    const sub = manager.onStateChange(state => {
      if (state === State.PoweredOn) {
        sub.remove();
        resolve(true);
      } else if (
        state === State.Unsupported ||
        state === State.Unauthorized
      ) {
        sub.remove();
        resolve(false);
      }
    }, true);
  });
}

// Scan for a device advertising BLE.DEVICE_NAME.
// Resolves with the found Device or null on timeout.
export function scanForDisplay(
  onStatusChange: (s: BleState) => void,
): Promise<Device | null> {
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      manager.stopDeviceScan();
      resolve(null);
    }, BLE.SCAN_TIMEOUT_MS);

    onStatusChange({ status: 'scanning', device: null, error: null });

    manager.startDeviceScan(null, null, (error, device) => {
      if (error) {
        clearTimeout(timeout);
        manager.stopDeviceScan();
        reject(error);
        return;
      }
      if (device?.name === BLE.DEVICE_NAME) {
        clearTimeout(timeout);
        manager.stopDeviceScan();
        resolve(device);
      }
    });
  });
}

// Connect to the device and return it (with services/chars discovered).
export async function connectToDevice(device: Device): Promise<Device> {
  const connected = await device.connect();
  return connected.discoverAllServicesAndCharacteristics();
}

// Serialize payload and write it chunk by chunk.
async function writeChunks(device: Device, data: string): Promise<void> {
  const encoder = new TextEncoder();
  const bytes = encoder.encode(data);
  let offset = 0;

  while (offset < bytes.length) {
    const chunk = bytes.slice(offset, offset + BLE.CHUNK_SIZE);
    const base64 = btoa(String.fromCharCode(...chunk));
    await device.writeCharacteristicWithResponseForService(
      BLE.SERVICE_UUID,
      BLE.QUOTES_CHAR_UUID,
      base64,
    );
    offset += BLE.CHUNK_SIZE;
  }
}

// Full sync: scan -> connect -> write.
// Calls onStatusChange at each step so the UI can reflect progress.
export async function syncToDisplay(
  quotes: QuotesResponse,
  onStatusChange: (s: BleState) => void,
): Promise<void> {
  const ready = await waitForBluetooth();
  if (!ready) {
    onStatusChange({ status: 'error', device: null, error: 'Bluetooth unavailable' });
    return;
  }

  let device: Device | null = null;
  try {
    device = await scanForDisplay(onStatusChange);
    if (!device) {
      onStatusChange({ status: 'error', device: null, error: 'Display not found (timeout)' });
      return;
    }

    onStatusChange({ status: 'connecting', device, error: null });
    const connected = await connectToDevice(device);
    onStatusChange({ status: 'connected', device: connected, error: null });

    const payload =
      `CHECKSUM:${quotes.checksum}\n` +
      JSON.stringify(quotes.quotes) +
      '\nEND\n';

    onStatusChange({ status: 'writing', device: connected, error: null });
    await writeChunks(connected, payload);

    onStatusChange({ status: 'done', device: connected, error: null });
  } catch (e: any) {
    onStatusChange({
      status: 'error',
      device,
      error: e?.message ?? 'Unknown BLE error',
    });
  } finally {
    if (device) {
      try { await device.cancelConnection(); } catch (_) {}
    }
  }
}
