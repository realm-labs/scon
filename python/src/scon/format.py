from __future__ import annotations

import re

from .error import SconError
from .value import SconArray, SconNumber, SconObject, SconValue


def format_value(value: SconValue) -> str:
    if not isinstance(value, SconObject):
        raise SconError("InvalidRootType", "SCON document root must be an object")
    return _format_object_body(value, 0) + "\n"


def _format_object_body(value: SconObject, indent: int) -> str:
    out = ""
    for key, item in value.items():
        out += " " * indent + _format_key(key) + " = " + _format_scon(item, indent) + "\n"
    return out


def _format_scon(value: SconValue, indent: int) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, str):
        return _quote(value, True)
    if isinstance(value, SconNumber):
        return value.to_text()
    if isinstance(value, SconArray):
        if not value:
            return "[]"
        body = "".join(" " * (indent + 2) + _format_scon(item, indent + 2) + ",\n" for item in value)
        return "[\n" + body + " " * indent + "]"
    if isinstance(value, SconObject):
        if not value:
            return "{}"
        return "{\n" + _format_object_body(value, indent + 2) + " " * indent + "}"
    raise SconError("Serde", f"unsupported SCON value: {type(value).__name__}")


def _format_key(key: str) -> str:
    return key if re.fullmatch(r"[A-Za-z_][A-Za-z0-9_-]*", key) else _quote(key, False)


def _quote(value: str, escape_interpolation: bool) -> str:
    out = '"'
    for ch in value:
        if ch == '"':
            out += '\\"'
        elif ch == "\\":
            out += "\\\\"
        elif ch == "\n":
            out += "\\n"
        elif ch == "\r":
            out += "\\r"
        elif ch == "\t":
            out += "\\t"
        elif ch == "\b":
            out += "\\b"
        elif ch == "\f":
            out += "\\f"
        elif ch == "$" and escape_interpolation:
            out += "\\$"
        elif ord(ch) < 0x20:
            out += f"\\u{ord(ch):04X}"
        else:
            out += ch
    return out + '"'
