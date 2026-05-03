/// AppRoute.swift — Navigation state for the main content area
///
/// Three states:
/// - .loading: checking bridge status on launch
/// - .web: bridge is running, show the WKWebView
/// - .diagnostics: bridge stopped or error, show diagnostics

import Foundation

enum AppRoute: Equatable {
    case loading
    case web
    case diagnostics
}
