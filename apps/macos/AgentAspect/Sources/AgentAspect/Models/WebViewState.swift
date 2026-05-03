/// WebViewState.swift — Loading state for BridgeWebView
///
/// Tracks the lifecycle of a WKWebView load:
/// - .idle: initial state before first load
/// - .loading: navigation in progress
/// - .loaded: page finished loading successfully
/// - .failed: navigation failed with an error description

import Foundation

enum WebViewState: Equatable {
    case idle
    case loading
    case loaded
    case failed(String)

    static func == (lhs: WebViewState, rhs: WebViewState) -> Bool {
        switch (lhs, rhs) {
        case (.idle, .idle), (.loading, .loading), (.loaded, .loaded):
            return true
        case (.failed(let a), .failed(let b)):
            return a == b
        default:
            return false
        }
    }

    var isFailed: Bool {
        if case .failed = self { return true }
        return false
    }
}
