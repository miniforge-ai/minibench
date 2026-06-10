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
            [babashka.process :as p]))

(def ^:private experiment "career.lens.acme-l4-eval")

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
      (tw "workbench:snapshot" "--mode" "lens" "--input" (input "career-lens-opus.edn")
          "--out" (out "experiments" "opus-semantic.json")
          "--experiment-id" experiment "--label" "opus+semantic"
          "--model" "claude-opus-4-8" "--method" "semantic")
      (tw "workbench:snapshot" "--mode" "lens" "--input" (input "career-lens-haiku.edn")
          "--out" (out "experiments" "haiku-mechanical.json")
          "--experiment-id" experiment "--label" "haiku+mechanical"
          "--model" "claude-haiku-4-5" "--method" "mechanical")
      ;; Portfolio — a second tenant, one readiness experiment under two
      ;; variants (healthy vs degraded inputs) so its matrix diverges too.
      (tw "workbench:portfolio-snapshot" "--readiness" (input "portfolio-readiness.json")
          "--out" (out "experiments" "portfolio-baseline.json")
          "--experiment-id" "portfolio.readiness" "--label" "baseline" "--model" "risk-pipeline")
      (tw "workbench:portfolio-snapshot" "--readiness" (input "portfolio-readiness-degraded.json")
          "--out" (out "experiments" "portfolio-degraded.json")
          "--experiment-id" "portfolio.readiness" "--label" "degraded" "--model" "risk-pipeline")
      (println "Regenerated fixtures from real adapter output.")
      (println "  cargo run -p minibench-cli -- compare fixtures/experiments"))))
