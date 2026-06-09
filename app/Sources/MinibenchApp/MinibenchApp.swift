// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import SwiftUI

/// Process entry point — one window rendering the comparison matrix from
/// the running data-plane. Start the data-plane first (`bb serve`); the
/// window loads on appear and via the toolbar Refresh.
@main
struct MinibenchApp: App {
    @State private var store = MatrixStore()

    var body: some Scene {
        WindowGroup(Strings.appTitle) {
            MatrixView()
                .environment(store)
                .frame(minWidth: Tokens.Window.minWidth, minHeight: Tokens.Window.minHeight)
                .task { await store.load() }
        }
        .windowResizability(.contentMinSize)
    }
}
