from __future__ import annotations

import enum
import math
from dataclasses import MISSING, fields, is_dataclass
from pathlib import Path
from types import UnionType
from typing import Any, TypeVar, get_args, get_origin, get_type_hints

from .error import SconError
from .format import format_value
from .resolver import parse_file, parse_string
from .value import SconArray, SconNumber, SconObject, SconValue

T = TypeVar("T")


def from_scon(source: str, typ: type[T]) -> T:
    return _decode(parse_string(source), typ)


def from_scon_file(path: str | Path, typ: type[T]) -> T:
    return _decode(parse_file(path), typ)


def to_scon(value: object) -> str:
    return format_value(_encode(value))


def _decode(value: SconValue, typ: Any) -> Any:
    origin = get_origin(typ)
    args = get_args(typ)
    if _is_optional(origin, args):
        if value is None:
            return None
        return _decode(value, next(arg for arg in args if arg is not type(None)))
    if typ is Any or typ is object:
        return _plain(value)
    if typ is type(None):
        if value is not None:
            raise SconError("Serde", "expected null")
        return None
    if typ is bool:
        if not isinstance(value, bool):
            raise SconError("Serde", "expected bool")
        return value
    if typ is str:
        if not isinstance(value, str):
            raise SconError("Serde", "expected string")
        return value
    if typ is int:
        if not isinstance(value, SconNumber):
            raise SconError("Serde", "expected number")
        return value.as_i64() if value.kind == "i64" else value.as_u64()
    if typ is float:
        if not isinstance(value, SconNumber):
            raise SconError("Serde", "expected number")
        return value.as_float()
    if isinstance(typ, type) and issubclass(typ, enum.Enum):
        if not isinstance(value, str):
            raise SconError("Serde", "expected enum string")
        return typ[value]
    if origin in (list, tuple):
        if not isinstance(value, SconArray):
            raise SconError("Serde", "expected array")
        item_type = args[0] if args else Any
        items = [_decode(item, item_type) for item in value]
        return tuple(items) if origin is tuple else items
    if origin is dict:
        if not isinstance(value, SconObject):
            raise SconError("Serde", "expected object")
        key_type = args[0] if args else str
        if key_type is not str:
            raise SconError("Serde", "SCON map keys must be strings")
        item_type = args[1] if len(args) > 1 else Any
        return {key: _decode(item, item_type) for key, item in value.items()}
    if isinstance(typ, type) and is_dataclass(typ):
        if not isinstance(value, SconObject):
            raise SconError("Serde", "expected object")
        kwargs = {}
        hints = get_type_hints(typ)
        for field in fields(typ):
            if field.name in value:
                kwargs[field.name] = _decode(value[field.name], hints.get(field.name, field.type))
            elif field.default is MISSING and field.default_factory is MISSING:
                raise SconError("Serde", f"missing field {field.name}")
        return typ(**kwargs)
    raise SconError("Serde", f"unsupported target type {typ}")


def _encode(value: object) -> SconValue:
    if value is None or isinstance(value, (bool, str)):
        return value
    if isinstance(value, int) and not isinstance(value, bool):
        if value < 0:
            if value < -(1 << 63):
                raise SconError("Serde", "integer overflow")
            return SconNumber("i64", value)
        if value > (1 << 64) - 1:
            raise SconError("Serde", "integer overflow")
        return SconNumber("u64", value)
    if isinstance(value, float):
        if not math.isfinite(value):
            raise SconError("Serde", "non-finite floats cannot be serialized")
        return SconNumber("f64", value)
    if isinstance(value, enum.Enum):
        return value.name
    if isinstance(value, (list, tuple)):
        return SconArray(_encode(item) for item in value)
    if isinstance(value, dict):
        out = SconObject()
        for key, item in value.items():
            if not isinstance(key, str):
                raise SconError("Serde", "SCON map keys must be strings")
            out[key] = _encode(item)
        return out
    if is_dataclass(value):
        out = SconObject()
        for field in fields(value):
            out[field.name] = _encode(getattr(value, field.name))
        return out
    raise SconError("Serde", f"unsupported value type {type(value).__name__}")


def _plain(value: SconValue) -> Any:
    if isinstance(value, SconNumber):
        return value.value
    if isinstance(value, SconArray):
        return [_plain(item) for item in value]
    if isinstance(value, SconObject):
        return {key: _plain(item) for key, item in value.items()}
    return value


def _is_optional(origin: Any, args: tuple[Any, ...]) -> bool:
    return (origin is UnionType or str(origin) == "typing.Union") and type(None) in args
