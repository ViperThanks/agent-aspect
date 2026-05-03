/// DateFormatters.swift — Shared date formatting utilities
///
/// Centralizes DateFormatter instances to avoid repeated allocation.
/// Used by various views when displaying timestamps from the bridge.

import Foundation

enum DateFormatters {
    /// ISO 8601 with fractional seconds (for bridge event timestamps)
    static let iso8601: ISO8601DateFormatter = {
        let formatter = ISO8601DateFormatter()
        formatter.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        return formatter
    }()

    /// Short display format: "May 4, 2026 3:42 PM"
    static let shortDisplay: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .medium
        formatter.timeStyle = .short
        return formatter
    }()

    /// Time-only format: "3:42 PM"
    static let timeOnly: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateStyle = .none
        formatter.timeStyle = .short
        return formatter
    }()
}
