;; Title: Minibench
;; Subtitle: evidence-validation gate over the committed fixtures
;; Author: Christopher Lester
;; Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

(ns validate
  "Run `minibench validate` over every committed snapshot (fixtures/*.json
   plus fixtures/baseline, fixtures/experiments, and
   fixtures/miniforge-etl/variants), pairing each snapshot with its
   registry by `registry_ref.registry_id` from
   fixtures/registries/<registry_id>.json. Discovery-driven: a new
   snapshot or tenant is gated automatically once its registry copy is
   committed, and a snapshot whose registry copy is MISSING fails the
   gate rather than being skipped silently.

   The registry copies are pinned duplicates of the product repos'
   registries; `validate` verifies registry_id/version/product against
   each snapshot's registry_ref, so a version bump upstream surfaces
   here as a hard error instead of a silently stale yardstick."
  (:require [babashka.fs :as fs]
            [babashka.process :as p]
            [cheshire.core :as json]))

(def ^:private snapshot-dirs
  ["fixtures"
   "fixtures/baseline"
   "fixtures/experiments"
   "fixtures/miniforge-etl/variants"])
(def ^:private registry-dir "fixtures/registries")

(defn- registry-for
  "Resolve the committed registry copy a snapshot names in its
   registry_ref. Returns {:registry path} or {:failure description} — an
   unreadable or unparseable snapshot, one with no registry_ref, and one
   whose registry copy is absent are all gate failures, reported per
   snapshot rather than thrown as a stacktrace that aborts the run."
  [snapshot-file]
  (let [registry-id (try
                      (-> (slurp snapshot-file)
                          (json/parse-string true)
                          (get-in [:registry_ref :registry_id]))
                      (catch Exception e
                        {::unreadable (ex-message e)}))]
    (cond
      (::unreadable registry-id)
      {:failure (str snapshot-file ": unreadable snapshot — "
                     (::unreadable registry-id))}

      (not (string? registry-id))
      {:failure (str snapshot-file ": no registry_ref.registry_id")}

      :else
      (let [path (fs/path registry-dir (str registry-id ".json"))]
        (if (fs/exists? path)
          {:registry (str path)}
          {:failure (str snapshot-file ": no committed registry copy in "
                         registry-dir)})))))

(defn- validate-snapshot!
  "Validate one snapshot against its registry with the prebuilt CLI at
   `binary`. Returns nil when clean, else a failure description."
  [binary snapshot-file]
  (let [{:keys [registry failure]} (registry-for snapshot-file)]
    (if failure
      failure
      (let [{:keys [exit]} (p/shell {:continue true}
                                    binary "validate" snapshot-file registry)]
        (when-not (zero? exit)
          (str snapshot-file ": evidence violations (exit " exit ")"))))))

(defn- cli-binary
  "Build the CLI once and return the path to it, so the gate spends one
   Cargo invocation rather than one per snapshot. Reads the target
   directory from Cargo rather than assuming ./target, which
   CARGO_TARGET_DIR can move."
  []
  (p/shell "cargo" "build" "-q" "-p" "minibench-cli")
  (let [target (-> (p/shell {:out :string}
                            "cargo" "metadata" "--format-version" "1" "--no-deps")
                   :out
                   (json/parse-string true)
                   :target_directory)
        binary (fs/path target "debug" "minibench")]
    (when-not (fs/exists? binary)
      (throw (ex-info (str "minibench CLI not found at " binary
                           " after cargo build")
                      {:binary (str binary)})))
    (str binary)))

(defn gate!
  "Validate every fixture snapshot; exit non-zero on any violation, any
   snapshot without a committed registry, and any snapshot that cannot
   be read."
  []
  (let [binary (cli-binary)
        snapshots (mapcat #(map str (fs/glob % "*.json")) snapshot-dirs)
        failures (into [] (keep #(validate-snapshot! binary %)) snapshots)]
    (println (format "validate-gate: %d snapshot(s) checked" (count snapshots)))
    (when (seq failures)
      (binding [*out* *err*]
        (doseq [failure failures] (println failure)))
      (System/exit 1))))
