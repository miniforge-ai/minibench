// Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

import SwiftUI

/// Process entry point — a two-pane shell: experiments sidebar + the
/// selected experiment's comparison matrix. Start the data-plane first
/// (`bb serve`); the list loads on appear and via the toolbar Refresh.
@main
struct MinibenchApp: App {
    @State private var store = AppStore()

    var body: some Scene {
        WindowGroup(Strings.appTitle) {
            ContentView()
                .environment(store)
                .frame(minWidth: Tokens.Window.minWidth, minHeight: Tokens.Window.minHeight)
        }
        .windowResizability(.contentMinSize)
    }
}
