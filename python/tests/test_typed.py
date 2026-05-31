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
