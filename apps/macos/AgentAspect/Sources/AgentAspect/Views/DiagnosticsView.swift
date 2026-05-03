/// DiagnosticsView.swift — Shown when the bridge is not running or has errors
///
/// Displays diagnostic info from BridgeStatusModel: data directory, binary path,
/// PID, address, LAN, launchd, keep-awake, token path, log and audit paths.
/// Primary action: "Start Bridge" via BridgeSupervisor. Secondary: "Run Doctor"
/// (placeholder for future milestones).

import SwiftUI

struct DiagnosticsView: View {
    @EnvironmentObject var appState: AppState

    var body: some View {
        VStack(spacing: 24) {
            headerSection
            diagnosticsGrid
            actionButtons
            Spacer()
        }
        .padding(32)
        .frame(maxWidth: 600)
    }

    // MARK: - Header

    private var headerSection: some View {
        VStack(spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill")
                .font(.system(size: 48))
                .foregroundColor(.orange)
            Text("Bridge Not Running")
                .font(.title)
            Text("The Agent Aspect bridge is not reachable. Start it to use the Web UI.")
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
        }
    }

    // MARK: - Diagnostics grid

    private var diagnosticsGrid: some View {
        GroupBox("Diagnostics") {
            VStack(alignment: .leading, spacing: 8) {
                diagnosticRow("Data Directory", value: appState.diagnostics.dataDir)
                diagnosticRow("Binary Path", value: appState.diagnostics.binaryPath)
                diagnosticRow("Bridge Status", value: appState.diagnostics.bridgeStatus)
                if !appState.diagnostics.pid.isEmpty {
                    diagnosticRow("PID", value: appState.diagnostics.pid)
                }
                if !appState.diagnostics.addr.isEmpty {
                    diagnosticRow("Address", value: appState.diagnostics.addr)
                }
                diagnosticRow("LAN", value: appState.diagnostics.lanEnabled)
                diagnosticRow("Launchd", value: appState.diagnostics.launchdLoaded)
                diagnosticRow("Keep-awake", value: appState.diagnostics.keepAwake)
                if !appState.diagnostics.tokenPath.isEmpty {
                    diagnosticRow("Token Path", value: appState.diagnostics.tokenPath)
                }
                diagnosticRow("Log File", value: appState.diagnostics.logFile)
                diagnosticRow("Audit DB", value: appState.diagnostics.auditDB)
                if let error = appState.diagnostics.error {
                    diagnosticRow("Error", value: error)
                }
            }
            .padding(.vertical, 8)
        }
    }

    private func diagnosticRow(_ label: String, value: String) -> some View {
        HStack(alignment: .top) {
            Text(label)
                .font(.system(.body, design: .monospaced))
                .foregroundColor(.secondary)
                .frame(width: 120, alignment: .trailing)
            Text(value)
                .font(.system(.body, design: .monospaced))
                .textSelection(.enabled)
            Spacer()
        }
    }

    // MARK: - Action buttons

    private var actionButtons: some View {
        HStack(spacing: 16) {
            Button(action: {
                appState.startBridge()
            }) {
                Label("Start Bridge", systemImage: "play.fill")
                    .frame(width: 160)
            }
            .buttonStyle(.borderedProminent)
            .controlSize(.large)

            Button(action: {
                // TODO(M41.3+): implement doctor runner
            }) {
                Label("Run Doctor", systemImage: "stethoscope")
                    .frame(width: 160)
            }
            .buttonStyle(.bordered)
            .controlSize(.large)
        }
    }
}
