// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import SwiftUI

/// L0 Foundations — design tokens for the Minibench shell, in the Miniforge
/// UX language (see `miniforge-control/app/.../DesignSystem.swift`). Per
/// `foundations/named-constants` (006): every meaningful literal is a named
/// token with a docstring. Plain data, no runtime cost.
enum Tokens {
    enum Window {
        /// Minimum window width — sidebar + the matrix (id column + two
        /// variant columns + spread) without truncation.
        static let minWidth: CGFloat = 900
        /// Minimum window height — header + a handful of state-var rows.
        static let minHeight: CGFloat = 460
    }

    enum Sidebar {
        /// Narrow enough to keep the eye on the matrix, wide enough for the
        /// dotted experiment ids + the variant-count subtitle.
        static let minWidth: CGFloat = 240
    }

    enum Spacing {
        /// Tight — within a cell (status badge ↔ score).
        static let tight: CGFloat = 4
        /// Normal — between grid rows / grouped controls (8pt grid step).
        static let normal: CGFloat = 16
        /// Roomy — between matrix columns, so variants read as columns.
        static let roomy: CGFloat = 24
    }

    enum Padding {
        /// Pane inset around the whole matrix.
        static let pane: CGFloat = 20
    }
}

/// Status → presentation. Status colors derive from the macOS system
/// semantic palette (per `design/ux-miniforge`: systemGreen / systemYellow /
/// systemRed) so they adapt to light/dark mode; muted label colors carry the
/// de-emphasized states.
extension StateStatus {
    var label: String {
        switch self {
        case .pass: Strings.statusPass
        case .warn: Strings.statusWarn
        case .fail: Strings.statusFail
        case .blocked: Strings.statusBlocked
        case .notApplicable: Strings.statusNotApplicable
        case .unknown: Strings.statusUnknown
        }
    }

    var tint: Color {
        switch self {
        case .pass: Color(.systemGreen)
        case .warn: Color(.systemYellow)
        case .fail: Color(.systemRed)
        case .blocked: Color(.secondaryLabelColor)
        case .notApplicable, .unknown: Color(.tertiaryLabelColor)
        }
    }
}
