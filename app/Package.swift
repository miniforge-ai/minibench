// swift-tools-version: 5.10
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import PackageDescription

// Minibench — native macOS shell (Miniforge UX). Slice 1: one window that
// renders the kernel ComparisonMatrix served by the data-plane on :8789.
// Pure SwiftPM, no external dependencies (SwiftUI + URLSession). FFI to the
// Rust core is deferred — the data-plane HTTP seam is minibench's chosen
// transport (see miniforge-control/app for the FFI pattern if ever needed).
let package = Package(
    name: "MinibenchApp",
    platforms: [
        // macOS 14 floor: the @Observable macro + @Environment(Type.self)
        // (Observation framework) require Sonoma.
        .macOS(.v14),
    ],
    targets: [
        .executableTarget(
            name: "MinibenchApp",
            path: "Sources/MinibenchApp"
        ),
    ]
)
