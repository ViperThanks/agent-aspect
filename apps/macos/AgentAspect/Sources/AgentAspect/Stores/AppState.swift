/// AppState.swift — Central observable state for the app
///
/// Owns the current route, bridge URL, status, and diagnostics info.
/// On initialize(), uses BridgeSupervisor to probe bridge status and
/// decide whether to show the Web view or Diagnostics view.

import Foundation
import Combine
import AppKit

final class AppState: ObservableObject {
    // MARK: - Published state

    @Published var route: AppRoute = .loading
    @Published var bridgeURL: URL?
    @Published var bridgeStatus: String = "unknown"
    @Published var diagnostics: DiagnosticsInfo = DiagnosticsInfo()
    @Published var reloadToken: UUID = UUID()
    @Published var webViewState: WebViewState = .idle

    // MARK: - Dependencies

    let bridgeSupervisor = BridgeSupervisor()

    // MARK: - Types

    struct DiagnosticsInfo {
        var dataDir: String = ""
        var binaryPath: String = ""
        var bridgeStatus: String = ""
        var logFile: String = ""
        var auditDB: String = ""
        var pid: String = ""
        var addr: String = ""
        var lanEnabled: String = ""
        var launchdLoaded: String = ""
        var keepAwake: String = ""
        var tokenPath: String = ""
        var error: String?
    }

    // MARK: - Lifecycle

    /// Called once from AgentAspectApp.onAppear.
    /// Probes bridge status and sets the initial route.
    func initialize() {
        diagnostics.dataDir = AgentAspectPaths.dataDir()
        diagnostics.logFile = AgentAspectPaths.daemonLogPath()
        diagnostics.auditDB = AgentAspectPaths.auditDBPath()

        // Locate binary
        diagnostics.binaryPath = bridgeSupervisor.binaryPath ?? "(not found)"

        // Check bridge status
        checkBridgeStatus()
    }

    /// Run `agent-aspect bridge status` via BridgeSupervisor and parse into a model.
    /// If bridge is running, read port and set route to .web.
    func checkBridgeStatus() {
        route = .loading

        let model = bridgeSupervisor.status()
        diagnostics.bridgeStatus = model.displaySummary
        bridgeStatus = model.displaySummary
        diagnostics.pid = model.pid.map { String($0) } ?? ""
        diagnostics.addr = model.addr ?? ""
        diagnostics.lanEnabled = model.lanEnabled ? "enabled" : "disabled"
        diagnostics.launchdLoaded = model.launchdLoaded ? "loaded" : "not loaded"
        diagnostics.keepAwake = model.keepAwake ? "enabled" : "disabled"
        diagnostics.tokenPath = model.tokenPath ?? ""

        if model.isRunning {
            if let port = bridgeSupervisor.readPort() {
                bridgeURL = URL(string: "http://127.0.0.1:\(port)/")
                route = .web
                return
            }
        }

        diagnostics.error = model.isRunning ? "running but port unknown" : model.displaySummary
        route = .diagnostics
    }

    /// Start the bridge via BridgeSupervisor (async), then re-check status.
    func startBridge() {
        bridgeSupervisor.start { [weak self] in
            self?.checkBridgeStatus()
        }
    }

    // MARK: - WebView actions

    /// Open the bridge URL in the system default browser.
    func openInBrowser() {
        guard let url = bridgeURL else { return }
        NSWorkspace.shared.open(url)
    }

    /// Signal the WebView to reload by toggling the reload token.
    func reloadWebView() {
        reloadToken = UUID()
    }
}
