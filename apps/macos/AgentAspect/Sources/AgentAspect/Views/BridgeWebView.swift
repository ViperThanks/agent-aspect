/// BridgeWebView.swift — WKWebView wrapper for the Bridge Web UI
///
/// Uses NSViewRepresentable (macOS, not UIViewRepresentable) to embed
/// a WKWebView that loads the bridge URL from AppState. Communicates
/// loading state back via a Binding<WebViewState>. Exposes the live
/// WKWebView reference so the parent can trigger reload or open-in-browser.

import SwiftUI
import WebKit

struct BridgeWebView: NSViewRepresentable {
    let url: URL
    @Binding var webViewState: WebViewState
    @Binding var webViewRef: WKWebView?

    func makeNSView(context: Context) -> WKWebView {
        let config = WKWebViewConfiguration()
        config.preferences.setValue(true, forKey: "developerExtrasEnabled")

        let webView = WKWebView(frame: .zero, configuration: config)
        webView.navigationDelegate = context.coordinator
        webView.allowsMagnification = true

        // Expose reference to parent
        DispatchQueue.main.async {
            self.webViewRef = webView
        }

        return webView
    }

    func updateNSView(_ nsView: WKWebView, context: Context) {
        // Only load if URL changed
        if nsView.url != url {
            webViewState = .loading
            let request = URLRequest(url: url)
            nsView.load(request)
        }
    }

    func dismantleNSView(_ nsView: WKWebView, coordinator: Coordinator) {
        DispatchQueue.main.async {
            self.webViewRef = nil
        }
    }

    func makeCoordinator() -> Coordinator {
        Coordinator(webViewState: $webViewState)
    }

    // MARK: - Coordinator

    final class Coordinator: NSObject, WKNavigationDelegate {
        @Binding var webViewState: WebViewState

        init(webViewState: Binding<WebViewState>) {
            _webViewState = webViewState
        }

        func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation!) {
            DispatchQueue.main.async {
                self.webViewState = .loading
            }
        }

        func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
            DispatchQueue.main.async {
                self.webViewState = .loaded
            }
        }

        func webView(
            _ webView: WKWebView,
            didFail navigation: WKNavigation!,
            withError error: Error
        ) {
            NSLog("BridgeWebView navigation failed: \(error.localizedDescription)")
            DispatchQueue.main.async {
                self.webViewState = .failed(error.localizedDescription)
            }
        }

        func webView(
            _ webView: WKWebView,
            didFailProvisionalNavigation navigation: WKNavigation!,
            withError error: Error
        ) {
            NSLog("BridgeWebView provisional navigation failed: \(error.localizedDescription)")
            DispatchQueue.main.async {
                self.webViewState = .failed(error.localizedDescription)
            }
        }
    }
}
