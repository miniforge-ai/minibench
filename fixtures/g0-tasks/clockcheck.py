"""SEALED — shared injected-time checker for the G0 tasks.

Every task injects time rather than reading a clock, so admission,
expiry, and revocation behaviour stay reproducible across replicates. A
candidate that consults wall time makes its own results unrepeatable, and
N15's replicate aggregation would read that variance as an effect.

Checking this by matching call shapes does not work. An earlier version
inspected `ast.Attribute` nodes with an `ast.Name` base, which sees
`time.time()` but misses `datetime.datetime.now()` (the base is itself an
Attribute) and misses `from time import time; time()` entirely (that is a
Call on a Name). Both are ordinary ways to read a clock.

So the check is on imports instead: a module that never imports a clock
cannot read one, whatever call shape it uses. The call-shape scan is kept
underneath as defence in depth, now covering chained attributes and bare
names, but the import ban is what actually closes the hole.
"""

import ast

CLOCK_MODULES = frozenset({"time", "datetime", "calendar", "sched", "zoneinfo"})

CLOCK_CALLS = frozenset({
    "time", "time_ns", "monotonic", "monotonic_ns", "perf_counter",
    "perf_counter_ns", "process_time", "thread_time", "sleep",
    "now", "utcnow", "today", "fromtimestamp", "utcfromtimestamp",
})


def _dotted(node):
    """Render an attribute chain as a dotted string, or None."""
    parts = []
    while isinstance(node, ast.Attribute):
        parts.append(node.attr)
        node = node.value
    if isinstance(node, ast.Name):
        parts.append(node.id)
        return ".".join(reversed(parts))
    return None


def find_clock_reads(src_dir):
    """Every way `src_dir` could reach a clock. Empty means it cannot."""
    findings = []
    for path in sorted(src_dir.glob("*.py")):
        tree = ast.parse(path.read_text())
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    if alias.name.split(".")[0] in CLOCK_MODULES:
                        findings.append(
                            "{}: imports {}".format(path.name, alias.name))
            elif isinstance(node, ast.ImportFrom) and node.module:
                if node.module.split(".")[0] in CLOCK_MODULES:
                    names = ", ".join(alias.name for alias in node.names)
                    findings.append(
                        "{}: imports {} from {}".format(
                            path.name, names, node.module))
            elif isinstance(node, ast.Call):
                target = node.func
                dotted = _dotted(target)
                if dotted and dotted.split(".")[-1] in CLOCK_CALLS:
                    root = dotted.split(".")[0]
                    if root in CLOCK_MODULES or "." in dotted:
                        findings.append("{}: calls {}()".format(path.name, dotted))
                elif isinstance(target, ast.Name) and target.id in CLOCK_CALLS:
                    findings.append(
                        "{}: calls {}()".format(path.name, target.id))
    return findings
