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

(defn- registry-path
  "The committed registry copy a snapshot names in its registry_ref, or
   nil when no copy exists for that registry id."
  [snapshot-file]
  (let [registry-id (-> (slurp snapshot-file)
                        (json/parse-string true)
                        (get-in [:registry_ref :registry_id]))
        path (fs/path registry-dir (str registry-id ".json"))]
    (when (fs/exists? path) (str path))))

(defn- validate-snapshot!
  "Validate one snapshot against its registry. Returns nil when clean,
   else a failure description."
  [snapshot-file]
  (if-let [registry (registry-path snapshot-file)]
    (let [{:keys [exit]} (p/shell {:continue true}
                                  "cargo" "run" "-q" "-p" "minibench-cli" "--"
                                  "validate" snapshot-file registry)]
      (when-not (zero? exit)
        (str snapshot-file ": evidence violations (exit " exit ")")))
    (str snapshot-file ": no committed registry copy in " registry-dir)))

(defn gate!
  "Validate every fixture snapshot; exit non-zero on any violation or
   any snapshot without a committed registry."
  []
  (let [snapshots (mapcat #(map str (fs/glob % "*.json")) snapshot-dirs)
        failures (into [] (keep validate-snapshot!) snapshots)]
    (println (format "validate-gate: %d snapshot(s) checked" (count snapshots)))
    (when (seq failures)
      (binding [*out* *err*]
        (doseq [failure failures] (println failure)))
      (System/exit 1))))
