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
    
    var body: some View {
        VStack(spacing: 32) {
            Spacer()
            
            // Quote display area placeholder
            VStack(spacing: 8) {
                Image(systemName: "quote.opening")
                    .font(.title)
                    .foregroundStyle(.secondary)
                
                Text("Tap fetch to load a quote")
                    .font(.body)
                    .foregroundStyle(.secondary)
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
    }
    
    private func fetchNewQuote() {
        // TODO: Implement fetch logic
        print("Fetching new quote...")
    }
}

// MARK: - Supporting Types

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
