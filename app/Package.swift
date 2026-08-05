// swift-tools-version: 5.10
// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai)
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
