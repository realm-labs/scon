from __future__ import annotations

from collections import OrderedDict
from dataclasses import dataclass
from math import isfinite
from typing import Literal

from .error import SconError

NumberKind = Literal["i64", "u64", "f64"]


@dataclass(frozen=True)
class SconNumber:
    kind: NumberKind
    value: int | float

    @staticmethod
    def parse(raw: str) -> SconNumber:
        try:
            if "." in raw or "e" in raw or "E" in raw:
                value = float(raw)
                if not isfinite(value):
                    raise ValueError("float value must be finite")
                return SconNumber("f64", value)
            value = int(raw, 10)
            if raw.startswith("-"):
                if value < -(1 << 63):
                    raise ValueError("signed integer is below i64 minimum")
                return SconNumber("i64", value)
            if value > (1 << 64) - 1:
                raise ValueError("unsigned integer exceeds u64 maximum")
            return SconNumber("u64", value)
        except ValueError as exc:
            raise SconError("InvalidNumber", f"invalid SCON number {raw!r}: {exc}") from exc

    @staticmethod
    def from_float(value: float) -> SconNumber:
        if not isfinite(value):
            raise SconError("Serde", "non-finite floats cannot be serialized")
        return SconNumber("f64", value)

    def as_i64(self) -> int:
        if self.kind == "i64":
            return int(self.value)
        if self.kind == "u64" and int(self.value) <= (1 << 63) - 1:
            return int(self.value)
        raise SconError("Serde", "integer overflow")

    def as_u64(self) -> int:
        if self.kind == "u64":
            return int(self.value)
        if self.kind == "i64" and int(self.value) >= 0:
            return int(self.value)
        raise SconError("Serde", "integer overflow")

    def as_float(self) -> float:
        return float(self.value)

    def to_text(self) -> str:
        if self.kind == "f64":
            return format(float(self.value), ".15g")
        return str(int(self.value))


class SconArray(list["SconValue"]):
    pass


class SconObject(OrderedDict[str, "SconValue"]):
    pass


SconValue = None | bool | str | SconNumber | SconArray | SconObject


def clone_value(value: SconValue) -> SconValue:
    if isinstance(value, SconArray):
        return SconArray(clone_value(item) for item in value)
    if isinstance(value, SconObject):
        out = SconObject()
        for key, item in value.items():
            out[key] = clone_value(item)
        return out
    return value
