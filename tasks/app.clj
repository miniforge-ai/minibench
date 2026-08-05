;; Title: Minibench
;; Subtitle: build the SwiftUI shell into a real .app bundle and launch it
;; Author: Christopher Lester
;; Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai)
;;
;; Licensed under the Apache License, Version 2.0 (the "License");
;; you may not use this file except in compliance with the License.
;; You may obtain a copy of the License at
;;
;;     http://www.apache.org/licenses/LICENSE-2.0
;;
;; Unless required by applicable law or agreed to in writing, software
;; distributed under the License is distributed on an "AS IS" BASIS,
;; WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
;; See the License for the specific language governing permissions and
;; limitations under the License.

(ns app
  "Build the SwiftUI shell into a real macOS `.app` bundle and launch it via
   `open`. A bare `swift run` executable never becomes a *regular* app — the
   window never appears and keyboard focus breaks (the
   feedback_app_bundle_launch operator lesson; miniforge-control's app even
   asserts a bundle context). The .app bundle is the required launch path."
  (:require [babashka.fs :as fs]
            [babashka.process :as p]
            [clojure.java.io :as io]
            [clojure.string :as str]
            [selmer.parser :as selmer]))

(def ^:private package-dir "app")
(def ^:private exec-name "MinibenchApp")
(def ^:private app-name "Minibench")
(def ^:private bundle-id "ai.miniforge.minibench")
(def ^:private copyright-line
  "Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). Licensed under Apache 2.0.")

;; Selmer template under tasks/ (a classpath root via bb.edn :paths), the
;; same pattern as miniforge-control. Selmer ships built-in with babashka.
(def ^:private info-plist-template "templates/Info.plist.selmer")

(defn- info-plist []
  (selmer/render (slurp (io/resource info-plist-template))
                 {:exec_name    exec-name
                  :bundle_id    bundle-id
                  :display_name app-name
                  :app_version  "dev-local"
                  :app_build    "dev-local"
                  :copyright    copyright-line}))

(defn build!
  "swift build the app package."
  []
  (p/shell "swift" "build" "--package-path" package-dir))

(defn- bin-path []
  (-> (p/shell {:out :string} "swift" "build" "--package-path" package-dir "--show-bin-path")
      :out
      str/trim))

(defn package!
  "Assemble `<build>/Minibench.app` around the built executable: Info.plist,
   any SwiftPM resource bundles, ad-hoc codesign. Returns the .app path."
  []
  (build!)
  (let [build-dir  (bin-path)
        built-exec (str build-dir "/" exec-name)
        app-dir    (str build-dir "/" app-name ".app")
        contents   (str app-dir "/Contents")
        macos      (str contents "/MacOS")
        resources  (str contents "/Resources")]
    (when-not (fs/exists? built-exec)
      (binding [*out* *err*]
        (println "swift build produced no executable at" built-exec))
      (System/exit 1))
    (println (str "==> assembling " app-dir))
    (when (fs/exists? app-dir) (fs/delete-tree app-dir))
    (doseq [d [macos resources]] (fs/create-dirs d))
    (fs/copy built-exec (str macos "/" exec-name))
    (fs/set-posix-file-permissions (str macos "/" exec-name) "rwxr-xr-x")
    ;; Carry SwiftPM-emitted resource bundles so Bundle.module lookups
    ;; still resolve from inside the .app.
    (doseq [bundle (fs/glob build-dir "*.bundle")]
      (fs/copy-tree (str bundle) (str resources "/" (fs/file-name bundle)) {:replace-existing true}))
    (spit (str contents "/Info.plist") (info-plist))
    (p/shell "plutil" "-lint" (str contents "/Info.plist"))
    ;; Ad-hoc sign so macOS treats it as a regular app (window + focus).
    (p/shell "codesign" "--force" "--sign" "-" "--deep" app-dir)
    app-dir))

(defn run-app!
  "Build + bundle + launch the .app via `open -n -W` (new instance, wait for
   exit). Start `bb serve` first for live data; otherwise the window shows
   the 'can't reach the data-plane' state."
  []
  (let [app-dir (package!)]
    (println (str "==> open -n -W " app-dir))
    (p/shell "open" "-n" "-W" app-dir)))
