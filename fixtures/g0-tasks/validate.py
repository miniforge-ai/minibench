#!/usr/bin/env python3
"""Calibration gate for the G0 task fixtures (N15 §6.3, §6.4).

A task qualifies for gate use only when its acceptance suite discriminates.
This script proves both halves of that:

1. **Solvability** — the author's reference solution passes the full suite.
   No LLM is involved; if the reference cannot pass, the task is unsolvable
   as written and no run result from it means anything.
2. **Distractor rejection** — every distractor FAILS at least one must-pass
   test. A suite that green-lights the literal reading of one stakeholder
   document is not measuring reconciliation, it is measuring compliance.
3. **Non-triviality** — the seeded repository, unmodified, FAILS the
   must-pass suite. A task whose starting state already passes measures
   nothing: every condition would score 1 and the cell would carry no signal.
4. **Sealed boundary** — nothing under `participant/` mentions the sealed
   material. A brief that names the reference solution or quotes a hidden
   test hands the answer to the run, and every result from that task after
   the leak is void.

Run: python3 fixtures/g0-tasks/validate.py [task-id ...]
"""

import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parent
MUST_PASS_FILES = ("test_resolution.py", "test_invariants.py", "test_regression.py")


def _stage(task_dir, candidate_dir):
    """Build a run tree: participant sources with the candidate overlaid."""
    staged = pathlib.Path(tempfile.mkdtemp(prefix="g0-"))
    shutil.copytree(task_dir / "participant", staged, dirs_exist_ok=True)
    for source in candidate_dir.glob("*.py"):
        shutil.copy(source, staged / "src" / source.name)
    shutil.copytree(task_dir / "sealed" / "acceptance", staged / "acceptance")
    (staged / "__init__.py").touch()
    (staged / "src" / "__init__.py").touch()
    (staged / "tests" / "__init__.py").touch()
    return staged


def _run_suite(staged, files):
    targets = [str(staged / "acceptance" / name) for name in files]
    completed = subprocess.run(
        [sys.executable, "-m", "pytest", "-q", "--no-header", *targets],
        cwd=staged,
        capture_output=True,
        text=True,
        env={"PYTHONPATH": "{}:{}".format(staged, staged / "acceptance"), "PATH": "/usr/bin:/bin"},
    )
    return completed.returncode == 0, completed.stdout + completed.stderr


def _all_suite_files(task_dir):
    return sorted(
        path.name
        for path in (task_dir / "sealed" / "acceptance").glob("test_*.py")
    )


LEAK_MARKERS = ("sealed/", "resolutions.md", "distractor", "reference solution")


def _check_sealed_boundary(task_dir):
    """Names of participant files that mention sealed material."""
    leaks = []
    for path in (task_dir / "participant").rglob("*"):
        if not path.is_file():
            continue
        try:
            text = path.read_text().lower()
        except UnicodeDecodeError:
            continue
        for marker in LEAK_MARKERS:
            if marker in text:
                leaks.append("{} mentions {!r}".format(
                    path.relative_to(task_dir), marker))
    return leaks


def validate(task_dir):
    """Return (ok, [lines]) for one task fixture."""
    lines = []
    ok = True

    leaks = _check_sealed_boundary(task_dir)
    if leaks:
        ok = False
        lines.append("  sealed boundary: FAILED — participant material leaks")
        lines.extend("    " + leak for leak in leaks)
    else:
        lines.append("  sealed boundary: participant material is clean")

    reference = task_dir / "sealed" / "reference"
    staged = _stage(task_dir, reference)
    passed, output = _run_suite(staged, _all_suite_files(task_dir))
    shutil.rmtree(staged, ignore_errors=True)
    if passed:
        lines.append("  solvability: reference passes the full suite")
    else:
        ok = False
        lines.append("  solvability: FAILED — reference does not pass")
        lines.append(_indent(output))

    staged = _stage(task_dir, task_dir / "participant" / "src")
    passed, _ = _run_suite(staged, MUST_PASS_FILES)
    shutil.rmtree(staged, ignore_errors=True)
    if passed:
        ok = False
        lines.append(
            "  non-triviality: FAILED — the seeded repository already passes"
        )
    else:
        lines.append("  non-triviality: seeded repository fails, as it must")

    distractors = sorted((task_dir / "sealed" / "distractors").glob("*"))
    if not distractors:
        ok = False
        lines.append("  distractors: FAILED — none supplied (N15 §6.4)")
    for distractor in distractors:
        staged = _stage(task_dir, distractor)
        passed, output = _run_suite(staged, MUST_PASS_FILES)
        shutil.rmtree(staged, ignore_errors=True)
        if passed:
            ok = False
            lines.append(
                "  distractor {}: FAILED — passed the must-pass suite".format(
                    distractor.name
                )
            )
        else:
            lines.append("  distractor {}: correctly rejected".format(distractor.name))
    return ok, lines


def _indent(text):
    return "\n".join("    " + line for line in text.strip().splitlines()[-25:])


def main(argv):
    requested = argv[1:]
    tasks = sorted(
        path
        for path in ROOT.iterdir()
        if path.is_dir() and (path / "task.json").exists()
    )
    if requested:
        tasks = [path for path in tasks if path.name in requested]
    if not tasks:
        print("no task fixtures found")
        return 1

    failures = 0
    for task_dir in tasks:
        meta = json.loads((task_dir / "task.json").read_text())
        print("{} (v{})".format(meta["task_id"], meta["version"]))
        ok, lines = validate(task_dir)
        print("\n".join(lines))
        if not ok:
            failures += 1
    print()
    print(
        "{}/{} task fixtures calibrated".format(len(tasks) - failures, len(tasks))
    )
    return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
