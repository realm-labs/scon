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
        scon.parse_string(source)
    except scon.SconError:
        return


def main() -> None:
    atheris.Setup(sys.argv, test_one_input)
    atheris.Fuzz()


if __name__ == "__main__":
    main()
