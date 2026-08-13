// Handles all communication with the marginilia backend.
// API key is read from react-native-keychain.
// Call setApiKey() once at first launch (from a settings screen or env).

import * as Keychain from 'react-native-keychain';

const KEYCHAIN_SERVICE = 'marginilia_api';

export interface Quote {
  body: string;
  author: string;
  work: string;
}

export interface QuotesResponse {
  checksum: string;
  quotes: Quote[];
}

export async function setApiKey(key: string): Promise<void> {
  await Keychain.setGenericPassword('apikey', key, { service: KEYCHAIN_SERVICE });
}

async function getApiKey(): Promise<string> {
  const creds = await Keychain.getGenericPassword({ service: KEYCHAIN_SERVICE });
  if (!creds) throw new Error('No API key stored. Call setApiKey() first.');
  return creds.password;
}

export async function fetchQuotes(baseUrl: string): Promise<QuotesResponse> {
  const apiKey = await getApiKey();
  const url = `${baseUrl}/quotes`;

  const res = await fetch(url, {
    headers: { 'x-api-key': apiKey },
  });

  if (!res.ok) {
    throw new Error(`Backend returned ${res.status}: ${res.statusText}`);
  }

  return res.json() as Promise<QuotesResponse>;
}
