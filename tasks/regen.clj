;; Title: Minibench
;; Subtitle: regenerate demo fixtures from the real workbench adapters
;; Author: Christopher Lester
;; Copyright 2025-2026 Christopher Lester (christopher@miniforge.ai). All rights reserved.

(ns regen
  "Regenerate minibench's demo fixtures from the REAL workbench adapters —
   the live tenant feed. Runs the thesium-workflows `bb workbench:*` tasks
   on the synthetic, non-personal inputs in fixtures/inputs/ and writes
   their validated workbench_snapshot/v1 into fixtures/.

   Needs a thesium-workflows checkout (set THESIUM_WORKFLOWS, default the
   sibling layout) + babashka. NEVER point this at real tenant data — the
   committed fixtures must stay synthetic."
  (:require [babashka.fs :as fs]
            [babashka.process :as p]
            [clojure.string :as str]))

(def ^:private experiment "career.lens.acme-l4-eval")

(defn- repo-root
  "Absolute path to the minibench repo root (regen's cwd) — the prefix
   stripped from generated provenance paths."
  []
  (str (fs/absolutize ".")))

(defn- sanitize!
  "Strip the absolute repo-root prefix from any path the adapters
   embedded in `path` (e.g. a source-provenance evidence quote), leaving
   a repo-relative path. Keeps committed fixtures deterministic across
   machines — an absolute, user-specific path both leaks the local
   environment and churns the diff on every regen."
  [path]
  (let [raw (slurp path)
        cleaned (str/replace raw (str (repo-root) "/") "")]
    (when (not= raw cleaned)
      (spit path cleaned))))

(defn- workflows-dir
  "Absolute path to the thesium-workflows checkout (the producer repo)."
  []
  (-> (or (System/getenv "THESIUM_WORKFLOWS")
          (str (fs/path ".." "thesium-career" "thesium-workflows")))
      fs/absolutize
      str))

(defn- input
  "Absolute path to a committed adapter input fixture."
  [file]
  (str (fs/absolutize (fs/path "fixtures" "inputs" file))))

(defn- out
  "Absolute path under fixtures/ (the bb task runs with a different cwd, so
   --out must be absolute)."
  [& parts]
  (str (fs/absolutize (apply fs/path "fixtures" parts))))

(defn fixtures!
  "Project the synthetic inputs through the real adapters into fixtures/."
  []
  (let [tw-dir (workflows-dir)]
    (when-not (fs/exists? tw-dir)
      (binding [*out* *err*]
        (println "thesium-workflows not found at:" tw-dir)
        (println "Set THESIUM_WORKFLOWS to your checkout."))
      (System/exit 1))
    (let [tw (fn [& args] (apply p/shell {:dir tw-dir} "bb" args))]
      ;; Career lens — two divergent variants of ONE experiment: the
      ;; permutation matrix (same task, different model/method -> pass vs fail).
      ;; --variant-inputs: each variant's input IS that variant's measured
      ;; report, so the snapshots must not claim identical-inputs
      ;; comparability (mixed source_hashes would fail kernel compare).
      (tw "workbench:snapshot" "--mode" "lens" "--input" (input "career-lens-opus.edn")
          "--out" (out "experiments" "opus-semantic.json")
          "--experiment-id" experiment "--label" "opus+semantic"
          "--model" "claude-opus-4-8" "--method" "semantic" "--variant-inputs")
      (tw "workbench:snapshot" "--mode" "lens" "--input" (input "career-lens-haiku.edn")
          "--out" (out "experiments" "haiku-mechanical.json")
          "--experiment-id" experiment "--label" "haiku+mechanical"
          "--model" "claude-haiku-4-5" "--method" "mechanical" "--variant-inputs")
      ;; Portfolio — a second tenant, one readiness experiment under two
      ;; variants (healthy vs degraded inputs) so its matrix diverges too.
      (tw "workbench:portfolio-snapshot" "--readiness" (input "portfolio-readiness.json")
          "--out" (out "experiments" "portfolio-baseline.json")
          "--experiment-id" "portfolio.readiness" "--label" "baseline" "--model" "risk-pipeline"
          "--variant-inputs")
      (tw "workbench:portfolio-snapshot" "--readiness" (input "portfolio-readiness-degraded.json")
          "--out" (out "experiments" "portfolio-degraded.json")
          "--experiment-id" "portfolio.readiness" "--label" "degraded" "--model" "risk-pipeline"
          "--variant-inputs")
      ;; The adapters record the absolute input path they were handed in
      ;; source-provenance quotes; strip the repo-root prefix so the
      ;; committed fixtures stay machine-independent.
      (run! sanitize! (map str (fs/glob (out "experiments") "*.json")))
      (println "Regenerated fixtures from real adapter output.")
      (println "  cargo run -p minibench-cli -- compare fixtures/experiments"))))
