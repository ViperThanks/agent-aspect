/// CommandRunner.swift — Non-interactive command execution via Process
///
/// Runs an external binary with arguments, captures stdout/stderr,
/// and returns the result with an exit status. Supports a configurable
/// timeout (default 10 seconds).

import Foundation

struct CommandResult {
    let stdout: String
    let stderr: String
    let exitCode: Int32
}

final class CommandRunner {

    /// Run a command with the given arguments and return its output.
    ///
    /// - Parameters:
    ///   - path: Absolute path to the executable.
    ///   - arguments: Command-line arguments (may be empty).
    ///   - timeout: Maximum wall-clock seconds to wait. Default 10.
    /// - Returns: A `CommandResult` containing stdout, stderr, and exit code.
    func run(_ path: String, arguments: [String] = [], timeout: TimeInterval = 10) -> CommandResult {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: path)
        process.arguments = arguments

        let outPipe = Pipe()
        let errPipe = Pipe()
        process.standardOutput = outPipe
        process.standardError = errPipe

        do {
            try process.run()
        } catch {
            return CommandResult(
                stdout: "",
                stderr: "failed to launch: \(error.localizedDescription)",
                exitCode: -1
            )
        }

        // Enforce timeout on a background queue.
        let timer = DispatchSource.makeTimerSource()
        timer.schedule(deadline: .now() + timeout)
        timer.setEventHandler { [weak process] in
            process?.terminate()
        }
        timer.resume()

        // Read pipes before waitUntilExit to avoid deadlock when output
        // exceeds the pipe buffer capacity (~64 KB on macOS).
        let outData = outPipe.fileHandleForReading.readDataToEndOfFile()
        let errData = errPipe.fileHandleForReading.readDataToEndOfFile()

        process.waitUntilExit()
        timer.cancel()

        return CommandResult(
            stdout: String(data: outData, encoding: .utf8) ?? "",
            stderr: String(data: errData, encoding: .utf8) ?? "",
            exitCode: process.terminationStatus
        )
    }
}
