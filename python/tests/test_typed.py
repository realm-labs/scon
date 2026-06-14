from __future__ import annotations

import enum
from dataclasses import dataclass

import pytest

import scon
from scon import SconError


class Mode(enum.Enum):
    fast = enum.auto()
    slow = enum.auto()


@dataclass
class Nested:
    enabled: bool


@dataclass
class Config:
    name: str
    port: int
    ratio: float
    tags: list[str]
    nested: Nested
    mode: Mode


def test_dataclass_decode_encode_round_trip():
    cfg = scon.from_scon(
        """
        name = "demo"
        port = 8080
        ratio = 1.5
        tags = ["a", "b"]
        nested { enabled = true }
        mode = "fast"
        """,
        Config,
    )
    assert cfg == Config("demo", 8080, 1.5, ["a", "b"], Nested(True), Mode.fast)
    assert scon.from_scon(scon.to_scon(cfg), Config) == cfg


def test_typed_errors():
    with pytest.raises(SconError):
        scon.to_scon({1: "bad"})
    with pytest.raises(SconError):
        scon.from_scon("name = 1", Config)


def test_analysis_and_source_formatter_preserve_source_constructs():
    source = "defaults { port = 8080 }\nserver = ${defaults.port}\nitems = [1, ...${extra}]\n"
    analysis = scon.analyze_source(source)
    assert [diagnostic.code for diagnostic in analysis.diagnostics] == ["MissingReference"]
    assert len(analysis.symbols) >= 3
    assert len(analysis.references) == 2

    formatted = scon.format_source(
        "# keep me\n"
        'include "base.scon"\n'
        "defaults { port = 8080 }\n"
        "server = ${defaults.port}\n"
        "items = [1, ...${extra}]\n"
    )
    assert scon.analyze_source(formatted).parsed is not None
    assert "# keep me" in formatted
    assert 'include "base.scon"' in formatted
    assert "...${extra}" in formatted
