import React, { useState, useCallback } from 'react';
import {
  View,
  Text,
  TouchableOpacity,
  StyleSheet,
  ActivityIndicator,
  SafeAreaView,
  Alert,
} from 'react-native';
import AsyncStorage from '@react-native-async-storage/async-storage';

import { fetchQuotes, QuotesResponse } from './api';
import { syncToDisplay, BleState, BleStatus } from './ble';
import { API, STORAGE } from './constants';

// ─── helpers ────────────────────────────────────────────────────────────────

function statusLabel(state: BleState): string {
  switch (state.status) {
    case 'idle':      return 'Ready';
    case 'scanning':  return 'Scanning for display…';
    case 'connecting':return `Connecting to ${state.device?.name ?? 'display'}…`;
    case 'connected': return `Connected to ${state.device?.name ?? 'display'}`;
    case 'writing':   return 'Sending quotes…';
    case 'done':      return 'Sync complete!';
    case 'error':     return `Error: ${state.error}`;
  }
}

function statusColor(status: BleStatus): string {
  switch (status) {
    case 'done':    return '#34C759'; // green
    case 'error':   return '#FF3B30'; // red
    case 'idle':    return '#8E8E93'; // grey
    default:        return '#007AFF'; // blue (in-progress)
  }
}

// ─── component ──────────────────────────────────────────────────────────────

export default function App(): React.JSX.Element {
  const [syncing, setSyncing] = useState(false);
  const [bleState, setBleState] = useState<BleState>({
    status: 'idle',
    device: null,
    error: null,
  });
  const [lastSync, setLastSync] = useState<string | null>(null);

  const handleSync = useCallback(async () => {
    setSyncing(true);
    setBleState({ status: 'idle', device: null, error: null });

    try {
      // 1. Fetch quotes from backend
      let quotes: QuotesResponse;
      try {
        quotes = await fetchQuotes(API.BASE_URL);
      } catch (e: any) {
        Alert.alert('Backend error', e?.message ?? 'Could not reach server');
        setSyncing(false);
        return;
      }

      // 2. Check if checksum changed since last sync
      const storedChecksum = await AsyncStorage.getItem(STORAGE.LAST_CHECKSUM);
      if (storedChecksum === quotes.checksum) {
        setBleState({ status: 'done', device: null, error: null });
        Alert.alert('Already up to date', 'Display has the latest quotes.');
        setSyncing(false);
        return;
      }

      // 3. Send to display over BLE
      await syncToDisplay(quotes, (s: BleState) => setBleState(s));

      // 4. Persist checksum and timestamp on success
      if (bleState.status !== 'error') {
        const now = new Date().toLocaleString();
        await AsyncStorage.setItem(STORAGE.LAST_CHECKSUM, quotes.checksum);
        await AsyncStorage.setItem(STORAGE.LAST_SYNC, now);
        setLastSync(now);
      }
    } finally {
      setSyncing(false);
    }
  }, [bleState.status]);

  const isInProgress = syncing && !['done', 'error'].includes(bleState.status);

  return (
    <SafeAreaView style={styles.container}>
      <View style={styles.content}>

        {/* Header */}
        <Text style={styles.title}>marginilia</Text>
        <Text style={styles.subtitle}>e-ink quote display</Text>

        {/* Display status card */}
        <View style={styles.card}>
          <Text style={styles.cardLabel}>Display</Text>
          <View style={styles.row}>
            <View
              style={[
                styles.dot,
                { backgroundColor: statusColor(bleState.status) },
              ]}
            />
            <Text style={[styles.statusText, { color: statusColor(bleState.status) }]}>
              {statusLabel(bleState)}
            </Text>
          </View>
          {bleState.device && (
            <Text style={styles.deviceName}>
              {bleState.device.name ?? bleState.device.id}
            </Text>
          )}
        </View>

        {/* Last sync */}
        {lastSync && (
          <Text style={styles.lastSync}>Last synced: {lastSync}</Text>
        )}

        {/* Sync button */}
        <TouchableOpacity
          style={[styles.button, isInProgress && styles.buttonDisabled]}
          onPress={handleSync}
          disabled={isInProgress}
          activeOpacity={0.7}
        >
          {isInProgress ? (
            <ActivityIndicator color="#fff" />
          ) : (
            <Text style={styles.buttonText}>Sync to Display</Text>
          )}
        </TouchableOpacity>

      </View>
    </SafeAreaView>
  );
}

// ─── styles ─────────────────────────────────────────────────────────────────

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#F2F2F7',
  },
  content: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 24,
    gap: 16,
  },
  title: {
    fontSize: 36,
    fontWeight: '700',
    color: '#1C1C1E',
    letterSpacing: -1,
  },
  subtitle: {
    fontSize: 15,
    color: '#8E8E93',
    marginTop: -8,
    marginBottom: 8,
  },
  card: {
    width: '100%',
    backgroundColor: '#FFFFFF',
    borderRadius: 16,
    padding: 20,
    gap: 8,
    shadowColor: '#000',
    shadowOpacity: 0.06,
    shadowRadius: 8,
    shadowOffset: { width: 0, height: 2 },
  },
  cardLabel: {
    fontSize: 12,
    fontWeight: '600',
    color: '#8E8E93',
    textTransform: 'uppercase',
    letterSpacing: 0.5,
  },
  row: {
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
  },
  dot: {
    width: 8,
    height: 8,
    borderRadius: 4,
  },
  statusText: {
    fontSize: 15,
    fontWeight: '500',
    flexShrink: 1,
  },
  deviceName: {
    fontSize: 13,
    color: '#8E8E93',
    marginLeft: 16,
  },
  lastSync: {
    fontSize: 13,
    color: '#8E8E93',
  },
  button: {
    marginTop: 16,
    width: '100%',
    backgroundColor: '#007AFF',
    paddingVertical: 16,
    borderRadius: 14,
    alignItems: 'center',
    justifyContent: 'center',
  },
  buttonDisabled: {
    backgroundColor: '#A2C4F5',
  },
  buttonText: {
    color: '#FFFFFF',
    fontSize: 17,
    fontWeight: '600',
  },
});
