//
//  ContentView.swift
//  marginilia
//
//  Created by Yaneury Fermin on 8/18/26.
//

import SwiftUI

struct ContentView: View {
    @State private var backendStatus: ConnectionStatus = .disconnected
    @State private var esp32Status: ConnectionStatus = .disconnected
    @State private var currentQuote: Quote?
    @State private var isLoading = false
    @State private var errorMessage: String?
    
    var body: some View {
        VStack(spacing: 32) {
            Spacer()
            
            // Quote display area
            VStack(spacing: 8) {
                if isLoading {
                    ProgressView()
                        .scaleEffect(1.5)
                } else if let quote = currentQuote {
                    VStack(spacing: 16) {
                        Image(systemName: "quote.opening")
                            .font(.title3)
                            .foregroundStyle(.secondary)
                        
                        Text(quote.text)
                            .font(.title3)
                            .multilineTextAlignment(.center)
                        
                        Text("— \(quote.author)")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
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
            
            Spacer()
            
            // Fetch button
            Button(action: fetchNewQuote) {
                Label("Fetch Quote", systemImage: "arrow.down.circle.fill")
                    .font(.headline)
                    .frame(maxWidth: .infinity)
                    .padding()
                    .background(.tint, in: RoundedRectangle(cornerRadius: 12))
                    .foregroundStyle(.white)
            }
            .buttonStyle(.plain)
            .disabled(isLoading)
            
            // Status indicators
            HStack(spacing: 24) {
                StatusIndicator(
                    title: "Backend",
                    status: backendStatus
                )
                
                StatusIndicator(
                    title: "ESP32",
                    status: esp32Status
                )
            }
            .padding(.top, 8)
        }
        .padding(24)
        .task {
            await checkBackendConnection()
        }
    }
    
    private func fetchNewQuote() {
        Task {
            await fetchQuote()
        }
    }
    
    private func fetchQuote() async {
        isLoading = true
        errorMessage = nil
        
        do {
            let quote = try await QuoteService.shared.fetchRandomQuote()
            currentQuote = quote
            backendStatus = .connected
        } catch {
            errorMessage = "Failed to fetch quote: \(error.localizedDescription)"
            backendStatus = .disconnected
        }
        
        isLoading = false
    }
    
    private func checkBackendConnection() async {
        do {
            _ = try await QuoteService.shared.fetchRandomQuote()
            backendStatus = .connected
        } catch {
            backendStatus = .disconnected
        }
    }
}

// MARK: - Supporting Types

struct Quote: Codable, Identifiable {
    let id: Int
    let text: String
    let author: String
}

enum ConnectionStatus {
    case connected
    case disconnected
    
    var color: Color {
        switch self {
        case .connected: return .green
        case .disconnected: return .red
        }
    }
}

struct StatusIndicator: View {
    let title: String
    let status: ConnectionStatus
    
    var body: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(status.color)
                .frame(width: 12, height: 12)
            
            Text(title)
                .font(.subheadline)
                .foregroundStyle(.secondary)
        }
    }
}

// MARK: - Networking

actor QuoteService {
    static let shared = QuoteService()
    
    private let baseURL = "http://workstation:3000"
    private let apiKey = "your-secret-key"
    
    private init() {}
    
    func fetchRandomQuote() async throws -> Quote {
        guard let url = URL(string: "\(baseURL)/quotes") else {
            throw URLError(.badURL)
        }
        
        var request = URLRequest(url: url)
        request.setValue(apiKey, forHTTPHeaderField: "x-api-key")
        request.timeoutInterval = 10
        
        let (data, response) = try await URLSession.shared.data(for: request)
        
        guard let httpResponse = response as? HTTPURLResponse else {
            throw URLError(.badServerResponse)
        }
        
        guard httpResponse.statusCode == 200 else {
            throw URLError(.init(rawValue: httpResponse.statusCode))
        }
        
        let quote = try JSONDecoder().decode(Quote.self, from: data)
        return quote
    }
}

// MARK: - Preview

#Preview {
    ContentView()
}

#Preview("Connected") {
    ContentView()
        .onAppear {
            // Preview with connected status - won't work with @State, just for visualization
        }
}
