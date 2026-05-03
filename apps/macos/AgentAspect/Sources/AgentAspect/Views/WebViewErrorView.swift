/// WebViewErrorView.swift — Error state for failed BridgeWebView loads
///
/// Displayed when the WKWebView fails to load the bridge URL.
/// Shows the error message, a retry button, and an "Open in Browser"
/// fallback that launches the URL in the system default browser.

import SwiftUI

struct WebViewErrorView: View {
    let errorMessage: String
    let onRetry: () -> Void
    let onOpenInBrowser: () -> Void

    var body: some View {
        VStack(spacing: 24) {
            Spacer()

            Image(systemName: "wifi.exclamationmark")
                .font(.system(size: 48))
                .foregroundColor(.red)

            Text("Failed to Load")
                .font(.title)

            Text("The bridge web interface could not be reached.")
                .font(.body)
                .foregroundColor(.secondary)

            Text(errorMessage)
                .font(.system(.body, design: .monospaced))
                .foregroundColor(.secondary)
                .textSelection(.enabled)
                .multilineTextAlignment(.center)
                .padding(.horizontal, 32)

            HStack(spacing: 16) {
                Button(action: onRetry) {
                    Label("Retry", systemImage: "arrow.clockwise")
                        .frame(width: 140)
                }
                .buttonStyle(.borderedProminent)
                .controlSize(.large)

                Button(action: onOpenInBrowser) {
                    Label("Open in Browser", systemImage: "safari")
                        .frame(width: 160)
                }
                .buttonStyle(.bordered)
                .controlSize(.large)
            }

            Spacer()
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(Color(nsColor: .windowBackgroundColor))
    }
}
