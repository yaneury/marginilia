import CoreBluetooth
import CryptoKit
import SwiftUI

// MARK: - Models

struct Quote: Codable {
    let body: String
    let author: String
    let work: String
    let createdAt: UInt64

    enum CodingKeys: String, CodingKey {
        case body, author, work
        case createdAt = "created_at"
    }
}

struct QuotesResponse: Codable {
    let latestTimestamp: UInt64
    let quotes: [Quote]

    enum CodingKeys: String, CodingKey {
        case latestTimestamp = "latest_timestamp"
        case quotes
    }
}

// MARK: - Backend

actor QuoteService {
    static let shared = QuoteService()

    private let baseURL = "http://workstation:3000"
    private let apiKey = "your-secret-key"

    private init() {}

    func fetchAll() async throws -> QuotesResponse {
        try await fetch(since: 0)
    }

    func fetch(since: UInt64) async throws -> QuotesResponse {
        var components = URLComponents(string: "\(baseURL)/quotes")!
        if since > 0 {
            components.queryItems = [URLQueryItem(name: "since", value: "\(since)")]
        }
        var request = URLRequest(url: components.url!)
        request.setValue(apiKey, forHTTPHeaderField: "x-api-key")
        request.timeoutInterval = 10
        let (data, response) = try await URLSession.shared.data(for: request)
        guard let http = response as? HTTPURLResponse, http.statusCode == 200 else {
            throw URLError(.badServerResponse)
        }
        return try JSONDecoder().decode(QuotesResponse.self, from: data)
    }
}

// MARK: - BLE

private let serviceUUID = CBUUID(string: "4a1b2c3d-0001-0000-0000-000000000000")
private let characteristicUUID = CBUUID(string: "4a1b2c3d-0002-0000-0000-000000000000")

enum BLEStatus: Equatable {
    case idle, scanning, connecting, syncing(Int, Int), done, failed(String)
}

@MainActor
class BLEManager: NSObject, ObservableObject {
    @Published var status: BLEStatus = .idle

    private var central: CBCentralManager!
    private var peripheral: CBPeripheral?
    private var characteristic: CBCharacteristic?
    private var chunks: [Data] = []
    private var chunkIndex = 0
    private var continuation: CheckedContinuation<Void, Error>?

    override init() {
        super.init()
        central = CBCentralManager(delegate: self, queue: .main)
    }

    func sync(quotes: [Quote]) async throws {
        struct DeviceQuote: Encodable { let body, author, work: String }
        let deviceQuotes = quotes.map { DeviceQuote(body: $0.body, author: $0.author, work: $0.work) }
        let json = try JSONEncoder().encode(deviceQuotes)
        let hex = SHA256.hash(data: json).map { String(format: "%02x", $0) }.joined()
        var payload = Data("CHECKSUM:\(hex)\n".utf8)
        payload.append(json)
        payload.append(Data("\nEND\n".utf8))

        chunks = stride(from: 0, to: payload.count, by: 20).map {
            payload[$0..<min($0 + 20, payload.count)]
        }
        chunkIndex = 0

        try await withCheckedThrowingContinuation { (c: CheckedContinuation<Void, Error>) in
            self.continuation = c
            self.status = .scanning
            self.central.scanForPeripherals(withServices: [serviceUUID])
        }
    }

    private func writeNext() {
        guard let p = peripheral, let ch = characteristic else { return }
        guard chunkIndex < chunks.count else {
            status = .done
            continuation?.resume()
            continuation = nil
            p.setNotifyValue(false, for: ch)
            central.cancelPeripheralConnection(p)
            return
        }
        status = .syncing(chunkIndex + 1, chunks.count)
        p.writeValue(chunks[chunkIndex], for: ch, type: .withResponse)
    }

    private func fail(_ error: Error) {
        status = .failed(error.localizedDescription)
        continuation?.resume(throwing: error)
        continuation = nil
    }
}

extension BLEManager: CBCentralManagerDelegate {
    func centralManagerDidUpdateState(_ central: CBCentralManager) {}

    func centralManager(_ central: CBCentralManager, didDiscover peripheral: CBPeripheral,
                        advertisementData: [String: Any], rssi _: NSNumber) {
        guard peripheral.name == "Marginilia" else { return }
        central.stopScan()
        self.peripheral = peripheral
        peripheral.delegate = self
        status = .connecting
        central.connect(peripheral)
    }

    func centralManager(_ central: CBCentralManager, didConnect peripheral: CBPeripheral) {
        peripheral.discoverServices([serviceUUID])
    }

    func centralManager(_ central: CBCentralManager, didFailToConnect _: CBPeripheral, error: Error?) {
        fail(error ?? URLError(.cannotConnectToHost))
    }

    func centralManager(_ central: CBCentralManager, didDisconnectPeripheral _: CBPeripheral, error: Error?) {
        if let error, continuation != nil { fail(error) }
    }
}

extension BLEManager: CBPeripheralDelegate {
    func peripheral(_ peripheral: CBPeripheral, didDiscoverServices error: Error?) {
        if let e = error { fail(e); return }
        guard let svc = peripheral.services?.first(where: { $0.uuid == serviceUUID }) else {
            fail(URLError(.cannotFindHost)); return
        }
        peripheral.discoverCharacteristics([characteristicUUID], for: svc)
    }

    func peripheral(_ peripheral: CBPeripheral, didDiscoverCharacteristicsFor service: CBService, error: Error?) {
        if let e = error { fail(e); return }
        guard let ch = service.characteristics?.first(where: { $0.uuid == characteristicUUID }) else {
            fail(URLError(.cannotFindHost)); return
        }
        characteristic = ch
        writeNext()
    }

    func peripheral(_ peripheral: CBPeripheral, didWriteValueFor characteristic: CBCharacteristic, error: Error?) {
        if let e = error { fail(e); return }
        chunkIndex += 1
        writeNext()
    }
}

// MARK: - View

struct ContentView: View {
    @State private var backendStatus: ConnectionStatus = .disconnected
    @State private var currentQuote: Quote?
    @State private var isLoading = false
    @State private var isSyncing = false
    @State private var errorMessage: String?
    @StateObject private var ble = BLEManager()

    var body: some View {
        VStack(spacing: 32) {
            Spacer()

            quoteArea

            Spacer()

            Button(action: { Task { await fetchQuote() } }) {
                Label("Fetch Quote", systemImage: "arrow.down.circle.fill")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(.tint, in: RoundedRectangle(cornerRadius: 12))
                    .foregroundStyle(.white)
            }
            .buttonStyle(.plain)
            .disabled(isLoading || isSyncing)

            Button(action: { Task { await syncToDevice() } }) {
                Label(syncLabel, systemImage: "wave.3.right")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(Color.secondary.opacity(0.15), in: RoundedRectangle(cornerRadius: 12))
                    .foregroundStyle(.primary)
            }
            .buttonStyle(.plain)
            .disabled(isLoading || isSyncing)

            HStack(spacing: 24) {
                StatusIndicator(title: "Backend", status: backendStatus)
                StatusIndicator(title: "ESP32", status: deviceStatus)
            }
            .padding(.top, 8)
        }
        .padding(24)
        .task { await fetchQuote() }
    }

    private var quoteArea: some View {
        Group {
            if isLoading || isSyncing {
                ProgressView()
                    .scaleEffect(1.5)
            } else if let quote = currentQuote {
                VStack(spacing: 16) {
                    Image(systemName: "quote.opening")
                        .font(.title3)
                        .foregroundStyle(.secondary)
                    Text(quote.body)
                        .font(.title3)
                        .multilineTextAlignment(.center)
                    Text("— \(quote.author)")
                        .font(.subheadline)
                        .foregroundStyle(.secondary)
                    if !quote.work.isEmpty {
                        Text(quote.work)
                            .font(.caption)
                            .foregroundStyle(.tertiary)
                            .italic()
                    }
                }
            } else if let error = errorMessage {
                VStack(spacing: 8) {
                    Image(systemName: "exclamationmark.triangle")
                        .font(.title)
                        .foregroundStyle(.orange)
                    Text(error)
                        .font(.body)
                        .foregroundStyle(.secondary)
                        .multilineTextAlignment(.center)
                }
            } else {
                VStack(spacing: 8) {
                    Image(systemName: "quote.opening")
                        .font(.title)
                        .foregroundStyle(.secondary)
                    Text("Tap fetch to load a quote")
                        .font(.body)
                        .foregroundStyle(.secondary)
                }
            }
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 48)
    }

    private var syncLabel: String {
        if case .syncing(let done, let total) = ble.status {
            return "Syncing \(done)/\(total)…"
        }
        return "Sync to Device"
    }

    private var deviceStatus: ConnectionStatus {
        switch ble.status {
        case .done: return .connected
        case .failed: return .disconnected
        default: return .disconnected
        }
    }

    private func fetchQuote() async {
        isLoading = true
        errorMessage = nil
        do {
            let response = try await QuoteService.shared.fetchAll()
            currentQuote = response.quotes.randomElement()
            backendStatus = .connected
        } catch {
            errorMessage = error.localizedDescription
            backendStatus = .disconnected
        }
        isLoading = false
    }

    private func syncToDevice() async {
        isSyncing = true
        errorMessage = nil
        do {
            let response = try await QuoteService.shared.fetchAll()
            try await ble.sync(quotes: response.quotes)
        } catch {
            errorMessage = "Sync failed: \(error.localizedDescription)"
        }
        isSyncing = false
    }
}

// MARK: - Supporting types

enum ConnectionStatus {
    case connected, disconnected

    var color: Color {
        self == .connected ? .green : .red
    }
}

struct StatusIndicator: View {
    let title: String
    let status: ConnectionStatus

    var body: some View {
        HStack(spacing: 8) {
            Circle().fill(status.color).frame(width: 12, height: 12)
            Text(title).font(.subheadline).foregroundStyle(.secondary)
        }
    }
}

#Preview { ContentView() }
