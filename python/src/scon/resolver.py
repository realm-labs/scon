from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path

from .error import SconError, Span
from .format import format_value
from .parser import (
    AstArray,
    AstArraySpread,
    AstArrayValue,
    AstBool,
    AstField,
    AstInclude,
    AstNull,
    AstNumber,
    AstObject,
    AstObjectSpread,
    AstObjectValue,
    AstString,
    AstSubstitution,
    StringInterpolationPart,
    StringLiteralPart,
    parse_document,
)
from .value import SconArray, SconNumber, SconObject, SconValue, clone_value


@dataclass
class LoadOptions:
    include_root: Path | None = None
    max_file_size: int = 16 * 1024 * 1024
    max_include_depth: int = 64
    max_include_files: int = 1024
    max_array_length: int = 1_000_000
    max_object_depth: int = 512


@dataclass
class EvalEntry:
    value: SconValue | EvalObject
    layer: str
    kind: str


class EvalObject:
    def __init__(self) -> None:
        self.entries: dict[str, EvalEntry] = {}


def parse_string(source: str) -> SconValue:
    return Resolver(LoadOptions()).eval(parse_document(source))


def parse_file(path: str | Path, *, include_root: str | Path | None = None) -> SconValue:
    file = Path(path).resolve()
    root = Path(include_root).resolve() if include_root is not None else file.parent
    source = _read_text_raw(file)
    options = LoadOptions(include_root=root)
    if len(source.encode()) > options.max_file_size:
        raise SconError("ResourceLimitExceeded", "maximum file size exceeded")
    resolver = Resolver(options)
    resolver.stack.append(file)
    resolver.seen.add(file)
    return resolver.eval(parse_document(source, str(file)))


def get_path(value: SconValue, path: str) -> SconValue:
    current = value
    for segment in path.split("."):
        if not isinstance(current, SconObject):
            raise SconError("TypeMismatch", "path segment requires object")
        if segment not in current:
            raise SconError("MissingReference", "path is not defined")
        current = current[segment]
    return current


class Resolver:
    def __init__(self, options: LoadOptions) -> None:
        self.options = options
        self.stack: list[Path] = []
        self.seen: set[Path] = set()
        self.cache = {}
        self.root = EvalObject()
        self.in_progress: list[list[str]] = [[]]

    def eval(self, document) -> SconValue:
        self._eval_object(document.root, [], document.file)
        return _public_object(self.root)

    def _eval_object(self, obj: AstObject, path: list[str], file: str | None) -> None:
        if len(path) > self.options.max_object_depth:
            raise SconError("ResourceLimitExceeded", "maximum object depth exceeded", obj.span)
        local_seen = False
        for member in obj.members:
            if isinstance(member, AstObjectSpread):
                if local_seen:
                    raise SconError("InvalidSpread", "object spread must appear before local members", member.span)
                target = self._lookup(member.sub.path, member.span).value
                if not isinstance(target, EvalObject):
                    raise SconError("TypeMismatch", "object spread target is not an object", member.span)
                _overlay_base(self._object_at(path, member.span), target)
            elif isinstance(member, AstInclude):
                included = self._load_include(file, member)
                self._eval_object(included.root, path, included.file)
            else:
                local_seen = True
                self._eval_field(member, path, file)

    def _eval_field(self, field: AstField, current: list[str], file: str | None) -> None:
        target = [*current, *[segment.value for segment in field.path.segments]]
        if isinstance(field.value, AstObjectValue):
            self._ensure_object(target, field.span)
            self.in_progress.append(target)
            try:
                self._eval_object(field.value.object, target, file)
            finally:
                self.in_progress.pop()
            return
        value = self._eval_value(field.value, file)
        self._insert(target, value, "ordinary", field.span)

    def _eval_value(self, value, file: str | None) -> SconValue | EvalObject:
        if isinstance(value, AstNull):
            return None
        if isinstance(value, AstBool):
            return value.value
        if isinstance(value, AstNumber):
            try:
                return SconNumber.parse(value.raw)
            except SconError as exc:
                raise SconError(exc.code, exc.message, value.span) from exc
        if isinstance(value, AstString):
            return self._eval_string(value)
        if isinstance(value, AstSubstitution):
            return _clone_any(self._lookup(value.path, value.span).value)
        if isinstance(value, AstArray):
            out = SconArray()
            self.in_progress.append([])  # prevents direct self-completed array lookup via lookup path check below.
            self.in_progress.pop()
            for item in value.items:
                if len(out) >= self.options.max_array_length:
                    raise SconError("ResourceLimitExceeded", "maximum array length exceeded", item.span)
                if isinstance(item, AstArrayValue):
                    out.append(_public_maybe(self._eval_value(item.value, file)))
                elif isinstance(item, AstArraySpread):
                    target = self._lookup(item.sub.path, item.span).value
                    if not isinstance(target, SconArray):
                        raise SconError("TypeMismatch", "array spread target is not an array", item.span)
                    out.extend(clone_value(v) for v in target)
            return out
        if isinstance(value, AstObjectValue):
            nested = Resolver(self.options)
            nested.stack = self.stack
            nested.seen = self.seen
            nested.cache = self.cache
            nested._eval_object(value.object, [], file)
            return nested.root
        raise SconError("UnexpectedToken", "unknown value")

    def _eval_string(self, value: AstString) -> str:
        if len(value.parts) == 1 and isinstance(value.parts[0], StringLiteralPart):
            return value.parts[0].value
        out = ""
        for part in value.parts:
            if isinstance(part, StringLiteralPart):
                out += part.value
            elif isinstance(part, StringInterpolationPart):
                replacement = self._lookup(part.path, part.span).value
                if isinstance(replacement, str):
                    out += replacement
                elif isinstance(replacement, bool):
                    out += "true" if replacement else "false"
                elif isinstance(replacement, SconNumber):
                    out += replacement.to_text()
                else:
                    raise SconError("TypeMismatch", "interpolation requires string, number, or boolean", part.span)
        return out

    def _lookup(self, path, span: Span) -> EvalEntry:
        names = [segment.value for segment in path.segments]
        if any(active == names for active in self.in_progress):
            raise SconError("MissingReference", "reference is not completed yet", span)
        obj = self.root
        entry = None
        for index, name in enumerate(names):
            entry = obj.entries.get(name)
            if entry is None:
                raise SconError("MissingReference", f"missing reference '{name}'", span)
            if index < len(names) - 1:
                if not isinstance(entry.value, EvalObject):
                    raise SconError("TypeMismatch", "reference path crosses non-object value", span)
                obj = entry.value
        assert entry is not None
        return entry

    def _ensure_object(self, path: list[str], span: Span) -> None:
        obj = self.root
        for index, name in enumerate(path):
            entry = obj.entries.get(name)
            if entry is None:
                child = EvalObject()
                obj.entries[name] = EvalEntry(child, "local", "structural")
                obj = child
                continue
            if not isinstance(entry.value, EvalObject):
                raise SconError("PathConflict", "path conflicts with scalar value", span)
            if index == len(path) - 1 and entry.layer == "local" and entry.kind != "structural":
                raise SconError("PathConflict", "object field conflicts with ordinary value", span)
            entry.layer = "local"
            entry.kind = "structural"
            obj = entry.value

    def _insert(self, path: list[str], value: SconValue | EvalObject, kind: str, span: Span) -> None:
        obj = self.root
        for name in path[:-1]:
            entry = obj.entries.get(name)
            if entry is None:
                child = EvalObject()
                obj.entries[name] = EvalEntry(child, "local", "structural")
                obj = child
            else:
                if not isinstance(entry.value, EvalObject):
                    raise SconError("PathConflict", "path conflicts with scalar value", span)
                obj = entry.value
        leaf = path[-1]
        existing = obj.entries.get(leaf)
        if existing is None:
            obj.entries[leaf] = EvalEntry(value, "local", kind)
            return
        if existing.layer == "base":
            _overlay_local(existing, value, kind)
            return
        raise SconError("DuplicateKey", f"duplicate key '{leaf}'", span)

    def _object_at(self, path: list[str], span: Span) -> EvalObject:
        obj = self.root
        for name in path:
            entry = obj.entries.get(name)
            if entry is None:
                raise SconError("PathConflict", "target object does not exist", span)
            if not isinstance(entry.value, EvalObject):
                raise SconError("PathConflict", "target path is not an object", span)
            obj = entry.value
        return obj

    def _load_include(self, file: str | None, include: AstInclude):
        path = include.path.value
        if _invalid_include_path(path):
            raise SconError("InvalidIncludePath", "invalid include path", include.span)
        root = (self.options.include_root or (Path(file).parent if file else Path("."))).resolve()
        base = Path(file).parent if file else root
        candidate = (base / path).resolve()
        if not _within_root(candidate, root):
            raise SconError("IncludePathDenied", "include path escapes include root", include.span)
        if candidate in self.stack:
            raise SconError("IncludeCycle", f"include cycle: {candidate}", include.span)
        if len(self.stack) >= self.options.max_include_depth:
            raise SconError("ResourceLimitExceeded", "maximum include depth exceeded", include.span)
        self.seen.add(candidate)
        if len(self.seen) > self.options.max_include_files:
            raise SconError("ResourceLimitExceeded", "maximum include file count exceeded", include.span)
        if candidate in self.cache:
            return self.cache[candidate]
        if not candidate.exists():
            raise SconError("IncludeNotFound", f"include file not found: {candidate}", include.span)
        if not candidate.is_file():
            raise SconError("IncludeNotFile", "include path is not a file", include.span)
        if candidate.stat().st_size > self.options.max_file_size:
            raise SconError("ResourceLimitExceeded", "maximum file size exceeded", include.span)
        self.stack.append(candidate)
        try:
            doc = parse_document(_read_text_raw(candidate), str(candidate))
            self.cache[candidate] = doc
            return doc
        except SconError as exc:
            code = "IncludeRootTypeError" if exc.code == "InvalidRootType" else "IncludeParseError"
            raise SconError(code, exc.message, exc.span) from exc
        finally:
            self.stack.pop()


def _public_object(obj: EvalObject) -> SconObject:
    out = SconObject()
    for key, entry in obj.entries.items():
        out[key] = _public_maybe(entry.value)
    return out


def _public_maybe(value: SconValue | EvalObject) -> SconValue:
    return _public_object(value) if isinstance(value, EvalObject) else value


def _clone_any(value: SconValue | EvalObject) -> SconValue | EvalObject:
    if isinstance(value, EvalObject):
        out = EvalObject()
        for key, entry in value.entries.items():
            out.entries[key] = EvalEntry(_clone_any(entry.value), entry.layer, entry.kind)
        return out
    return clone_value(value)


def _overlay_base(target: EvalObject, source: EvalObject) -> None:
    for key, entry in source.entries.items():
        existing = target.entries.get(key)
        if existing is None:
            target.entries[key] = EvalEntry(_clone_any(entry.value), "base", "ordinary")
        elif existing.layer == "base":
            _overlay_local(existing, _clone_any(entry.value), entry.kind)


def _overlay_local(existing: EvalEntry, value: SconValue | EvalObject, kind: str) -> None:
    if isinstance(existing.value, EvalObject) and isinstance(value, EvalObject):
        _merge_override(existing.value, value)
        existing.layer = "local"
        existing.kind = kind
    else:
        existing.value = value
        existing.layer = "local"
        existing.kind = kind


def _merge_override(target: EvalObject, source: EvalObject) -> None:
    for key, entry in source.entries.items():
        existing = target.entries.get(key)
        if existing is not None and isinstance(existing.value, EvalObject) and isinstance(entry.value, EvalObject):
            _merge_override(existing.value, entry.value)
        else:
            target.entries[key] = EvalEntry(_clone_any(entry.value), entry.layer, entry.kind)


def _invalid_include_path(path: str) -> bool:
    return (
        "://" in path
        or path.startswith("classpath:")
        or "*" in path
        or path.startswith("~")
        or path.startswith("$")
        or os.path.isabs(path)
        or (len(path) >= 3 and path[1] == ":" and path[2] in "\\/")
    )


def _within_root(path: Path, root: Path) -> bool:
    try:
        path.relative_to(root)
        return True
    except ValueError:
        return False


def _read_text_raw(path: Path) -> str:
    with path.open("r", encoding="utf-8", newline="") as file:
        return file.read()


__all__ = ["parse_string", "parse_file", "get_path", "format_value"]
