/// AgentAspectApp.swift — SwiftUI App entry point
///
/// Sets up the App lifecycle with a single main window.
/// Owns AppState as @StateObject so it survives window re-creation.

import SwiftUI

@main
struct AgentAspectApp: App {
    @StateObject private var appState = AppState()

    var body: some Scene {
        WindowGroup {
            ContentView()
                .environmentObject(appState)
                .frame(minWidth: 800, minHeight: 500)
                .onAppear {
                    appState.initialize()
                }
        }
    }
}
