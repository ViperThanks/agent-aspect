/// ContentView.swift — Root view that switches between Web and Diagnostics
///
/// Observes AppState.route to decide which view to show:
/// - .loading: a progress indicator while checking bridge
/// - .web: BridgeWebView loading the bridge URL, with loading/error overlays
/// - .diagnostics: DiagnosticsView with status info and start button
///
/// Toolbar items (Reload, Open in Browser) are visible only in the .web route.

import SwiftUI
import WebKit

struct ContentView: View {
    @EnvironmentObject var appState: AppState
    @State private var webViewRef: WKWebView?

    var body: some View {
        Group {
            switch appState.route {
            case .loading:
                loadingView

            case .web:
                if let url = appState.bridgeURL {
                    webViewContent(url: url)
                } else {
                    DiagnosticsView()
                }

            case .diagnostics:
                DiagnosticsView()
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .toolbar {
            if appState.route == .web {
                toolbarContent
            }
        }
    }

    // MARK: - Web view with overlays

    @ViewBuilder
    private func webViewContent(url: URL) -> some View {
        ZStack {
            BridgeWebView(
                url: url,
                webViewState: $appState.webViewState,
                webViewRef: $webViewRef
            )
            .ignoresSafeArea()

            // Loading overlay
            if appState.webViewState == .loading {
                loadingOverlay
            }

            // Error overlay
            if case .failed(let message) = appState.webViewState {
                WebViewErrorView(
                    errorMessage: message,
                    onRetry: { webViewRef?.reload() },
                    onOpenInBrowser: { appState.openInBrowser() }
                )
            }
        }
        .onChange(of: appState.reloadToken) { _ in
            webViewRef?.reload()
        }
    }

    // MARK: - Subviews

    private var loadingView: some View {
        VStack(spacing: 16) {
            ProgressView()
                .progressViewStyle(.circular)
            Text("Checking bridge status...")
                .font(.headline)
                .foregroundColor(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var loadingOverlay: some View {
        VStack(spacing: 12) {
            ProgressView()
                .progressViewStyle(.circular)
            Text("Loading...")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(20)
        .background(.ultraThinMaterial, in: RoundedRectangle(cornerRadius: 12))
    }

    // MARK: - Toolbar

    @ToolbarContentBuilder
    private var toolbarContent: some ToolbarContent {
        ToolbarItem(placement: .automatic) {
            Button(action: { webViewRef?.reload() }) {
                Label("Reload", systemImage: "arrow.clockwise")
            }
            .help("Reload the web page")
        }
        ToolbarItem(placement: .automatic) {
            Button(action: { appState.openInBrowser() }) {
                Label("Open in Browser", systemImage: "safari")
            }
            .help("Open in default browser")
        }
    }
}
