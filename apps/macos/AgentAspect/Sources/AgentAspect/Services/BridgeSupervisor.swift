/// BridgeSupervisor.swift — Manages the Agent Aspect bridge process lifecycle
///
/// Wraps the CLI subcommands (`bridge status`, `start`, `stop`) via
/// CommandRunner + BinaryLocator, and provides direct HTTP health checks
/// plus file-based port / state reads.

import Foundation

final class BridgeSupervisor {

    private let runner = CommandRunner()
    private let locator = BinaryLocator()

    /// Full path to the located binary, or nil.
    var binaryPath: String? { locator.locateBinary()?.path }

    // MARK: - CLI-backed operations

    /// Run `agent-aspect bridge status` and parse the output into a model.
    func status() -> BridgeStatusModel {
        let raw = runBridge("status")
        return BridgeStatusModel.parse(raw)
    }

    /// Run `agent-aspect bridge start` on a background queue, then call
    /// `completion` on the main thread after the bridge has had time to bind.
    func start(completion: @escaping () -> Void) {
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            guard let self else { return }
            _ = self.runBridge("start")
            // Give the bridge a moment to bind the port.
            Thread.sleep(forTimeInterval: 2)
            DispatchQueue.main.async {
                completion()
            }
        }
    }

    /// Synchronous start — for non-UI callers only. Blocks the calling thread.
    @discardableResult
    func startSync() -> BridgeStatusModel {
        _ = runBridge("start")
        Thread.sleep(forTimeInterval: 2)
        return status()
    }

    /// Run `agent-aspect bridge stop`.
    @discardableResult
    func stop() -> CommandResult {
        guard let binary = locator.locateBinary() else {
            return CommandResult(stdout: "", stderr: "binary not found", exitCode: -1)
        }
        return runner.run(binary.path, arguments: ["bridge", "stop"])
    }

    // TODO(M41.4): wire restart() to menu bar / settings UI
    /// Stop then start the bridge.
    func restart(completion: @escaping () -> Void) {
        _ = stop()
        start(completion: completion)
    }

    // MARK: - HTTP health

    // TODO(M41.4): wire health() to menu bar status indicator
    /// HTTP GET `http://127.0.0.1:<port>/health`. Returns true if status 200.
    func health() -> Bool {
        guard let port = readPort() else { return false }
        guard let url = URL(string: "http://127.0.0.1:\(port)/health") else { return false }

        let semaphore = DispatchSemaphore(value: 0)
        var ok = false

        let task = URLSession.shared.dataTask(with: url) { _, response, _ in
            if let http = response as? HTTPURLResponse, http.statusCode == 200 {
                ok = true
            }
            semaphore.signal()
        }
        task.resume()
        _ = semaphore.wait(timeout: .now() + 5)
        return ok
    }

    // MARK: - File reads

    /// Read the bridge port from `bridge.port` in the data directory.
    func readPort() -> Int? {
        let path = AgentAspectPaths.bridgePortPath()
        guard let content = try? String(contentsOfFile: path, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines),
              let port = Int(content) else {
            return nil
        }
        return port
    }

    // TODO(M41.4): wire readState() to launchd path repair diagnostics
    /// Read and parse `bridge.state.json` from the data directory.
    func readState() -> [String: Any]? {
        let path = AgentAspectPaths.bridgeStatePath()
        guard let data = FileManager.default.contents(atPath: path) else { return nil }
        guard let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return nil
        }
        return obj
    }

    // MARK: - Private helpers

    private func runBridge(_ subcommand: String) -> String {
        guard let binary = locator.locateBinary() else {
            return "error: agent-aspect binary not found"
        }
        let result = runner.run(binary.path, arguments: ["bridge", subcommand])
        return result.stdout.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
