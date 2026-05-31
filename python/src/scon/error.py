from __future__ import annotations

from dataclasses import dataclass
from typing import Literal

ErrorCode = Literal[
    "InvalidCharacter",
    "InvalidWhitespace",
    "InvalidEscape",
    "UnexpectedToken",
    "UnterminatedString",
    "InvalidNumber",
    "InvalidRootType",
    "DuplicateKey",
    "PathConflict",
    "MissingReference",
    "TypeMismatch",
    "InvalidSpread",
    "InvalidIncludePath",
    "IncludeNotFound",
    "IncludeNotFile",
    "IncludePathDenied",
    "IncludeCycle",
    "IncludeParseError",
    "IncludeRootTypeError",
    "ResourceLimitExceeded",
    "Serde",
]


@dataclass(frozen=True)
class Span:
    start: int
    end: int


class SconError(Exception):
    def __init__(self, code: ErrorCode, message: str, span: Span | None = None) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message
        self.span = span


def fail(code: ErrorCode, message: str, start: int, end: int) -> None:
    raise SconError(code, message, Span(start, end))
