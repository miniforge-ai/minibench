"""Suite shipped with the Bellwether gateway. SYNTHETIC fixture.

These assertions describe behaviour that must hold before and after any
change to the limiter.
"""

from src.config import parse_limits
from src.limiter import Limiter


def config(global_rate=800, per_user_rate=10, **extra):
    base = {"global_rate": global_rate, "per_user_rate": per_user_rate}
    base.update(extra)
    return base


def test_parses_the_documented_keys():
    parsed = parse_limits("global_rate: 800\nper_user_rate: 10\n")
    assert parsed["global_rate"] == 800
    assert parsed["per_user_rate"] == 10


def test_ignores_comments_and_blank_lines():
    parsed = parse_limits("# heading\n\nglobal_rate: 500  # inline\nper_user_rate: 5\n")
    assert parsed == {"global_rate": 500, "per_user_rate": 5}


def test_rejects_a_file_missing_required_keys():
    try:
        parse_limits("global_rate: 800\n")
    except ValueError:
        return
    raise AssertionError("expected a ValueError for a missing required key")


def test_admits_traffic_under_the_ceiling():
    limiter = Limiter(config(global_rate=10))
    assert all(limiter.allow("u1", 0.0) for _ in range(10))


def test_sheds_past_the_global_ceiling():
    limiter = Limiter(config(global_rate=10))
    for _ in range(10):
        limiter.allow("u1", 0.0)
    assert limiter.allow("u1", 0.0) is False


def test_window_rolls_after_one_second():
    limiter = Limiter(config(global_rate=10))
    for _ in range(10):
        limiter.allow("u1", 0.0)
    assert limiter.allow("u1", 0.0) is False
    assert limiter.allow("u1", 1.0) is True


def test_reports_admissions_in_the_current_window():
    limiter = Limiter(config(global_rate=10))
    for _ in range(4):
        limiter.allow("u1", 0.0)
    assert limiter.admitted_in_window() == 4
