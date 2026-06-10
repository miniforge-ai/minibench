// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation

/// L0 — every user-facing string + the data-plane endpoint, centralized per
/// `foundations/named-constants` (006) and `languages/swift` (240). Full
/// i18n (`Localizable.strings` per locale) is a later slice; these are the
/// extraction point for it.
enum Strings {
    static let appTitle = "Minibench"

    /// The data-plane comparison route on loopback (risk :8787, career
    /// :8788, minibench :8789).
    static let comparisonEndpoint = "http://127.0.0.1:8789/v1/comparison"

    // Phase / chrome
    static let loading = "Loading comparison…"
    static let refresh = "Refresh"
    static let emptyMatrix = "No experiment loaded."

    // Errors
    static let errorTitle = "Comparison unavailable"
    static let errorBadEndpoint = "The comparison endpoint URL is invalid."
    static let errorBadStatus = "The data-plane returned an unexpected status."
    static let errorUnreachable = "Can't reach the data-plane on :8789. Start it with `bb serve`."
    static let errorBadPayload = "The data-plane responded, but the comparison payload didn't decode."

    // Matrix columns / marks
    static let colStateVar = "State variable"
    static let colSpread = "Spread"
    static let absentCell = "—"
    static let divergeMark = "◆"

    // Status labels (the contract's StateStatus)
    static let statusPass = "pass"
    static let statusWarn = "warn"
    static let statusFail = "fail"
    static let statusBlocked = "blocked"
    static let statusNotApplicable = "n/a"
    static let statusUnknown = "unknown"
}
