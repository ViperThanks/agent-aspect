// swift-tools-version: 5.9
// Package.swift — AgentAspect macOS app (SwiftPM)
//
// A native macOS shell that wraps the existing Bridge Web UI in a WKWebView.
// Minimum deployment: macOS 13.0 (Ventura). No external dependencies.

import PackageDescription

let package = Package(
    name: "AgentAspect",
    platforms: [
        .macOS(.v13)
    ],
    products: [
        .executable(name: "AgentAspect", targets: ["AgentAspect"])
    ],
    targets: [
        .executableTarget(
            name: "AgentAspect",
            path: "Sources/AgentAspect",
            resources: [
                .copy("../../Resources/Binaries")
            ],
            linkerSettings: [
                .linkedFramework("WebKit")
            ]
        )
    ]
)
