/// AppDelegate.swift — NSApplicationDelegate hook (future use)
///
/// Reserved for M41.2: launchd integration, Keychain access, and
/// application lifecycle events. Currently a no-op placeholder.

import Cocoa

final class AppDelegate: NSObject, NSApplicationDelegate {
    func applicationDidFinishLaunching(_ notification: Notification) {
        // M41.2: launchd registration, Keychain warm-up
    }

    func applicationWillTerminate(_ notification: Notification) {
        // M41.2: graceful bridge stop if we own the process
    }
}
