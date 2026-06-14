from __future__ import annotations

import sys
from pathlib import Path

import atheris

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

with atheris.instrument_imports():
    import scon


def test_one_input(data: bytes) -> None:
    try:
        source = data.decode("utf-8")
    except UnicodeDecodeError:
        return
    try:
        formatted = scon.format_source(source)
    except scon.SconError:
        return

    analysis = scon.analyze_source(formatted)
    assert analysis.parsed is not None

    try:
        original = scon.parse_string(source)
        round_trip = scon.parse_string(formatted)
    except scon.SconError:
        return
    assert original == round_trip


def main() -> None:
    atheris.Setup(sys.argv, test_one_input)
    atheris.Fuzz()


if __name__ == "__main__":
    main()
