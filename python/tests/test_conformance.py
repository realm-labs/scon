from __future__ import annotations

import json
from pathlib import Path

import pytest

import scon
from scon import SconArray, SconError, SconNumber, SconObject


ROOT = Path(__file__).resolve().parents[2]
CONFORMANCE = ROOT / "tests" / "conformance"


def _cases():
    manifest = json.loads((CONFORMANCE / "manifest.json").read_text())
    return manifest["cases"]


@pytest.mark.parametrize("case", _cases(), ids=lambda case: case["id"])
def test_conformance(case):
    entry = CONFORMANCE / case["entry"]
    if case["kind"] == "valid":
        value = scon.parse_file(entry)
        expected = json.loads((CONFORMANCE / case["expected"]).read_text())
        assert _to_json(value) == expected
        assert _to_json(scon.parse_string(scon.format_value(value))) == expected
    else:
        expected = json.loads((CONFORMANCE / case["expected"]).read_text())
        with pytest.raises(SconError) as exc:
            scon.parse_file(entry)
        assert exc.value.code == expected["code"]


def _to_json(value):
    if isinstance(value, SconNumber):
        return value.value
    if isinstance(value, SconArray):
        return [_to_json(item) for item in value]
    if isinstance(value, SconObject):
        return {key: _to_json(item) for key, item in value.items()}
    return value
