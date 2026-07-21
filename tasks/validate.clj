;; Title: Minibench
;; Subtitle: evidence-validation gate over the committed fixtures
;; Author: Christopher Lester
;; Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

(ns validate
  "Run `minibench validate` over every committed adapter-produced
   snapshot (fixtures/baseline + fixtures/experiments), pairing each
   snapshot with its registry by `registry_ref.registry_id` from
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

(def ^:private snapshot-dirs ["fixtures/baseline" "fixtures/experiments"])
(def ^:private registry-dir "fixtures/registries")

(def ^:private violation-exit-code
  "minibench validate's exit code for evidence violations (CLI
   strings::VIOLATION_EXIT_CODE). Any OTHER non-zero exit is a hard
   validate failure — registry mismatch, an undecodable snapshot, a
   build error — not a violation, and must not be relabeled as one."
  4)

(defn- registry-id
  "The registry id a snapshot names in its registry_ref, or ::unparseable
   when the file is not JSON carrying that field. Parsing here rather
   than letting an exception escape keeps one bad fixture a reported
   failure instead of an aborted gate."
  [snapshot-file]
  (try
    (-> (slurp snapshot-file)
        (json/parse-string true)
        (get-in [:registry_ref :registry_id]))
    (catch Exception _ ::unparseable)))

(defn- registry-path
  "The committed registry copy for `registry-id`, or nil when no copy
   exists."
  [registry-id]
  (let [path (fs/path registry-dir (str registry-id ".json"))]
    (when (fs/exists? path) (str path))))

(defn- classify-exit
  "Classify a `minibench validate` exit code for one snapshot: nil when
   clean, else a failure map `{:kind :message}`. Exit 4 is evidence
   violations; any other non-zero is a hard validate failure."
  [snapshot-file exit]
  (cond
    (zero? exit) nil
    (= violation-exit-code exit)
    {:kind :violations
     :message (str snapshot-file ": evidence violations (exit " exit ")")}
    :else
    {:kind :error
     :message (str snapshot-file ": validate failed (exit " exit ")")}))

(defn- validate-snapshot!
  "Validate one snapshot against its registry. Returns nil when clean,
   else a failure map `{:kind :message}`. `:kind` is `:violations`
   (exit 4), `:error` (any other non-zero validate exit or an
   undecodable snapshot), or `:no-registry` (no committed registry
   copy)."
  [snapshot-file]
  (let [rid (registry-id snapshot-file)]
    (cond
      (= ::unparseable rid)
      {:kind :error
       :message (str snapshot-file ": snapshot is not decodable JSON with a registry_ref")}

      :else
      (if-let [registry (registry-path rid)]
        (let [{:keys [exit]} (p/shell {:continue true}
                                      "cargo" "run" "-q" "-p" "minibench-cli" "--"
                                      "validate" snapshot-file registry)]
          (classify-exit snapshot-file exit))
        {:kind :no-registry
         :message (str snapshot-file ": no committed registry copy in " registry-dir
                       " for " rid)}))))

(defn gate!
  "Validate every fixture snapshot; exit non-zero on any failure. The
   exit code is validate's own 4 when every failure is an evidence
   violation, so callers can tell violations apart from hard failures
   (a missing registry copy, an undecodable snapshot, a non-4 validate
   exit), which exit 1."
  []
  (let [snapshots (mapcat #(map str (fs/glob % "*.json")) snapshot-dirs)
        failures (into [] (keep validate-snapshot!) snapshots)]
    (println (format "validate-gate: %d snapshot(s) checked" (count snapshots)))
    (when (seq failures)
      (binding [*out* *err*]
        (doseq [{:keys [message]} failures] (println message)))
      (System/exit (if (every? #(= :violations (:kind %)) failures)
                     violation-exit-code
                     1)))))
