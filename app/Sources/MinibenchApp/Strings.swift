// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import Foundation

/// L0 — every user-facing string + the data-plane endpoint, centralized per
/// `foundations/named-constants` (006) and `languages/swift` (240). Full
/// i18n (`Localizable.strings` per locale) is a later slice; these are the
/// extraction point for it.
enum Strings {
    static let appTitle = "Minibench"

    // Phase / chrome
    static let loading = "Loading…"
    static let refresh = "Refresh"
    static let retry = "Retry"
    static let emptyMatrix = "No experiment loaded."

    // Errors
    static let errorTitle = "Unavailable"
    static let errorBadEndpoint = "The endpoint URL is invalid."
    static let errorBadStatus = "The data-plane returned an unexpected status."
    static let errorUnreachable = "Can't reach the data-plane on :8789. Start it with `bb serve`."
    static let errorBadPayload = "The data-plane responded, but the payload didn't decode."

    // Sidebar / detail
    static let selectExperiment = "Select an experiment"
    static let noExperiments = "No experiments loaded. Start the data-plane with `bb serve`."
    static let variantsSuffix = "variants"

    // Matrix columns / marks
    static let colStateVar = "State variable"
    static let colSpread = "Spread"
    static let colWithin = "Within"
    static let colSignals = "Signals"
    static let absentCell = "—"
    static let divergeMark = "◆"
    static let confidencePrefix = "c"
    static let missingSourceHashes = "source hashes missing"
    static let missingPolicyProvenance = "policy provenance missing"
    static let missingEvaluatorProvenance = "evaluator provenance missing"
    static let unknownWarning = "unknown warning"
    static let signalStatus = "status"
    static let signalCoverage = "coverage"
    static let signalUnstable = "unstable"

    // Status labels (the contract's StateStatus)
    static let statusPass = "pass"
    static let statusWarn = "warn"
    static let statusFail = "fail"
    static let statusBlocked = "blocked"
    static let statusNotApplicable = "n/a"
    static let statusUnknown = "unknown"
}
