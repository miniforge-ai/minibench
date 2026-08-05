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
