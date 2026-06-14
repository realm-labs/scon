from __future__ import annotations

from dataclasses import dataclass

from .error import ErrorCode, SconError, Span
from .lexer import Token, lex
from .parser import (
    AstArray,
    AstArraySpread,
    AstArrayValue,
    AstField,
    AstInclude,
    AstObject,
    AstObjectSpread,
    AstObjectValue,
    AstPath,
    AstString,
    AstSubstitution,
    AstValue,
    StringInterpolationPart,
    parse_document,
)
from .resolver import parse_string
from .value import SconValue


@dataclass(frozen=True)
class SourcePosition:
    line: int
    column: int


@dataclass(frozen=True)
class SourceRange:
    start: SourcePosition
    end: SourcePosition
    span: Span


@dataclass(frozen=True)
class Comment:
    text: str
    span: Span
    range: SourceRange


@dataclass(frozen=True)
class Diagnostic:
    code: ErrorCode
    message: str
    severity: str
    file: str | None
    range: SourceRange | None


@dataclass(frozen=True)
class TokenInfo:
    kind: str
    text: str
    span: Span
    range: SourceRange


@dataclass(frozen=True)
class Symbol:
    path: list[str]
    file: str | None
    range: SourceRange


@dataclass(frozen=True)
class Definition:
    path: list[str]
    file: str | None
    range: SourceRange


@dataclass
class Reference:
    path: list[str]
    kind: str
    file: str | None
    range: SourceRange
    target: Definition | None = None


@dataclass(frozen=True)
class IncludeReference:
    path: str
    file: str | None
    range: SourceRange
    resolved_path: str | None = None


@dataclass(frozen=True)
class ParsedSource:
    file: str | None
    tokens: list[TokenInfo]
    comments: list[Comment]
    symbols: list[Symbol]


@dataclass(frozen=True)
class Analysis:
    file: str | None
    parsed: ParsedSource | None
    diagnostics: list[Diagnostic]
    comments: list[Comment]
    symbols: list[Symbol]
    definitions: list[Definition]
    references: list[Reference]
    includes: list[IncludeReference]
    value: SconValue | None


class LineIndex:
    def __init__(self, source: str) -> None:
        self.lines = [0]
        for index, char in enumerate(source):
            if char == "\n":
                self.lines.append(index + 1)

    def range(self, span: Span) -> SourceRange:
        return SourceRange(self.position(span.start), self.position(span.end), span)

    def position(self, offset: int) -> SourcePosition:
        line = 0
        while line + 1 < len(self.lines) and self.lines[line + 1] <= offset:
            line += 1
        return SourcePosition(line, offset - self.lines[line])


def parse_source(source: str, file: str | None = None) -> ParsedSource:
    document = parse_document(source, file)
    line_index = LineIndex(source)
    tokens = _tokens(source, line_index)
    return ParsedSource(
        file=file,
        tokens=tokens,
        comments=_comments(tokens),
        symbols=_symbols(document.root, line_index, file, []),
    )


def analyze_source(source: str, file: str | None = None) -> Analysis:
    line_index = LineIndex(source)
    tokens = _safe_tokens(source, line_index)
    try:
        document = parse_document(source, file)
    except SconError as err:
        return Analysis(
            file=file,
            parsed=None,
            diagnostics=[_diagnostic(err, line_index, file)],
            comments=_comments(tokens),
            symbols=[],
            definitions=[],
            references=[],
            includes=[],
            value=None,
        )
    parsed = ParsedSource(
        file=file,
        tokens=tokens,
        comments=_comments(tokens),
        symbols=_symbols(document.root, line_index, file, []),
    )
    definitions = _definitions(document.root, line_index, file, [])
    references = _references(document.root, line_index, file)
    _resolve_targets(references, definitions)
    diagnostics: list[Diagnostic] = []
    value: SconValue | None = None
    try:
        value = parse_string(source)
    except SconError as err:
        diagnostics.append(_diagnostic(err, line_index, file))
    return Analysis(
        file=file,
        parsed=parsed,
        diagnostics=diagnostics,
        comments=parsed.comments,
        symbols=parsed.symbols,
        definitions=definitions,
        references=references,
        includes=_includes(document.root, line_index, file),
        value=value,
    )


def _tokens(source: str, line_index: LineIndex) -> list[TokenInfo]:
    return [TokenInfo(token.kind, token.text, token.span, line_index.range(token.span)) for token in lex(source)]


def _safe_tokens(source: str, line_index: LineIndex) -> list[TokenInfo]:
    try:
        return _tokens(source, line_index)
    except SconError:
        return []


def _comments(tokens: list[TokenInfo]) -> list[Comment]:
    return [Comment(token.text, token.span, token.range) for token in tokens if token.kind == "comment"]


def _symbols(obj: AstObject, line_index: LineIndex, file: str | None, prefix: list[str]) -> list[Symbol]:
    out: list[Symbol] = []
    for member in obj.members:
        if isinstance(member, AstField):
            path = [*prefix, *_path_names(member.path)]
            out.append(Symbol(path, file, line_index.range(member.path.span)))
            if isinstance(member.value, AstObjectValue):
                out.extend(_symbols(member.value.object, line_index, file, path))
    return out


def _definitions(obj: AstObject, line_index: LineIndex, file: str | None, prefix: list[str]) -> list[Definition]:
    out: list[Definition] = []
    for member in obj.members:
        if isinstance(member, AstField):
            path = [*prefix, *_path_names(member.path)]
            out.append(Definition(path, file, line_index.range(member.path.span)))
            if isinstance(member.value, AstObjectValue):
                out.extend(_definitions(member.value.object, line_index, file, path))
    return out


def _references(obj: AstObject, line_index: LineIndex, file: str | None) -> list[Reference]:
    out: list[Reference] = []
    for member in obj.members:
        if isinstance(member, AstObjectSpread):
            out.append(_reference(member.sub.path, "objectSpread", line_index, file))
        elif isinstance(member, AstField):
            out.extend(_value_references(member.value, line_index, file))
    return out


def _value_references(value: AstValue, line_index: LineIndex, file: str | None) -> list[Reference]:
    if isinstance(value, AstSubstitution):
        return [_reference(value.path, "substitution", line_index, file)]
    if isinstance(value, AstString):
        return [
            _reference(part.path, "interpolation", line_index, file)
            for part in value.parts
            if isinstance(part, StringInterpolationPart)
        ]
    if isinstance(value, AstArray):
        out: list[Reference] = []
        for item in value.items:
            if isinstance(item, AstArraySpread):
                out.append(_reference(item.sub.path, "arraySpread", line_index, file))
            elif isinstance(item, AstArrayValue):
                out.extend(_value_references(item.value, line_index, file))
        return out
    if isinstance(value, AstObjectValue):
        return _references(value.object, line_index, file)
    return []


def _includes(obj: AstObject, line_index: LineIndex, file: str | None) -> list[IncludeReference]:
    out: list[IncludeReference] = []
    for member in obj.members:
        if isinstance(member, AstInclude):
            out.append(IncludeReference(member.path.value, file, line_index.range(member.span)))
        elif isinstance(member, AstField) and isinstance(member.value, AstObjectValue):
            out.extend(_includes(member.value.object, line_index, file))
    return out


def _reference(path: AstPath, kind: str, line_index: LineIndex, file: str | None) -> Reference:
    return Reference(_path_names(path), kind, file, line_index.range(path.span))


def _resolve_targets(references: list[Reference], definitions: list[Definition]) -> None:
    by_path = {"\0".join(definition.path): definition for definition in definitions}
    for reference in references:
        reference.target = by_path.get("\0".join(reference.path))


def _path_names(path: AstPath) -> list[str]:
    return [segment.value for segment in path.segments]


def _diagnostic(error: SconError, line_index: LineIndex, file: str | None) -> Diagnostic:
    return Diagnostic(error.code, error.message, "error", file, line_index.range(error.span) if error.span else None)
