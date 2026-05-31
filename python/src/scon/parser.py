from __future__ import annotations

from dataclasses import dataclass

from .error import SconError, Span
from .lexer import Token, lex


@dataclass(frozen=True)
class Document:
    root: AstObject
    file: str | None = None


@dataclass(frozen=True)
class AstObject:
    members: list[AstMember]
    span: Span


@dataclass(frozen=True)
class AstPathSegment:
    value: str
    quoted: bool
    span: Span


@dataclass(frozen=True)
class AstPath:
    segments: list[AstPathSegment]
    span: Span


@dataclass(frozen=True)
class AstField:
    path: AstPath
    value: AstValue
    span: Span
    type: str = "field"


@dataclass(frozen=True)
class AstInclude:
    path: AstString
    span: Span
    type: str = "include"


@dataclass(frozen=True)
class AstObjectSpread:
    sub: AstSubstitution
    span: Span
    type: str = "object_spread"


@dataclass(frozen=True)
class AstNull:
    span: Span
    type: str = "null"


@dataclass(frozen=True)
class AstBool:
    value: bool
    span: Span
    type: str = "bool"


@dataclass(frozen=True)
class AstNumber:
    raw: str
    span: Span
    type: str = "number"


@dataclass(frozen=True)
class StringLiteralPart:
    value: str
    type: str = "literal"


@dataclass(frozen=True)
class StringInterpolationPart:
    path: AstPath
    span: Span
    type: str = "interpolation"


StringPart = StringLiteralPart | StringInterpolationPart


@dataclass(frozen=True)
class AstString:
    value: str
    raw: str
    parts: list[StringPart]
    span: Span
    type: str = "string"


@dataclass(frozen=True)
class AstArrayValue:
    value: AstValue
    span: Span
    type: str = "value"


@dataclass(frozen=True)
class AstArraySpread:
    sub: AstSubstitution
    span: Span
    type: str = "spread"


AstArrayItem = AstArrayValue | AstArraySpread


@dataclass(frozen=True)
class AstArray:
    items: list[AstArrayItem]
    span: Span
    type: str = "array"


@dataclass(frozen=True)
class AstObjectValue:
    object: AstObject
    span: Span
    type: str = "object"


@dataclass(frozen=True)
class AstSubstitution:
    path: AstPath
    span: Span
    type: str = "substitution"


AstMember = AstField | AstInclude | AstObjectSpread
AstValue = AstNull | AstBool | AstNumber | AstString | AstArray | AstObjectValue | AstSubstitution


def parse_document(source: str, file: str | None = None) -> Document:
    return Document(Parser(lex(source)).parse(), file)


class Parser:
    def __init__(self, tokens: list[Token]) -> None:
        self.tokens = tokens
        self.index = 0

    def parse(self) -> AstObject:
        self._skip_trivia()
        if self._match("{"):
            root = self._parse_object(self._previous())
        elif self._check("["):
            raise SconError("InvalidRootType", "SCON document root must be an object", self._peek().span)
        else:
            root = self._parse_object_body(self._peek().span.start)
        self._skip_trivia()
        self._expect("eof", "expected end of file")
        return root

    def _parse_object(self, opening: Token) -> AstObject:
        members = self._parse_members("}")
        closing = self._expect("}", "expected '}'")
        return AstObject(members, Span(opening.span.start, closing.span.end))

    def _parse_object_body(self, start: int) -> AstObject:
        members = self._parse_members("eof")
        end = members[-1].span.end if members else start
        return AstObject(members, Span(start, end))

    def _parse_members(self, end: str) -> list[AstMember]:
        members: list[AstMember] = []
        self._skip_trivia()
        while not self._check(end) and not self._check("eof"):
            members.append(self._parse_member())
            self._skip_trivia()
            if self._match(","):
                self._skip_trivia()
                if self._check(","):
                    raise SconError("UnexpectedToken", "consecutive commas are invalid", self._peek().span)
        return members

    def _parse_member(self) -> AstMember:
        self._skip_trivia()
        if self._match("include"):
            include = self._previous()
            self._skip_inline_trivia()
            path = self._parse_string()
            if any(isinstance(part, StringInterpolationPart) for part in path.parts):
                raise SconError("UnexpectedToken", "include path must be a literal string", path.span)
            return AstInclude(path, Span(include.span.start, path.span.end))
        if self._match("..."):
            spread = self._previous()
            sub = self._parse_substitution()
            return AstObjectSpread(sub, Span(spread.span.start, sub.span.end))
        path = self._parse_path()
        self._skip_inline_trivia()
        if self._match("="):
            self._skip_inline_trivia()
            if self._check("newline"):
                raise SconError("UnexpectedToken", "field value cannot start on the next line", self._peek().span)
            value = self._parse_value()
        elif self._match("{"):
            obj = self._parse_object(self._previous())
            value = AstObjectValue(obj, obj.span)
        else:
            raise SconError("UnexpectedToken", "expected '=' or object shorthand", self._peek().span)
        return AstField(path, value, Span(path.span.start, value.span.end))

    def _parse_value(self) -> AstValue:
        self._skip_trivia()
        if self._match("null"):
            return AstNull(self._previous().span)
        if self._match("true"):
            return AstBool(True, self._previous().span)
        if self._match("false"):
            return AstBool(False, self._previous().span)
        if self._match("number"):
            token = self._previous()
            return AstNumber(token.text, token.span)
        if self._check("string"):
            return self._parse_string()
        if self._match("{"):
            obj = self._parse_object(self._previous())
            return AstObjectValue(obj, obj.span)
        if self._match("["):
            return self._parse_array(self._previous())
        if self._check("subst"):
            return self._parse_substitution()
        raise SconError("UnexpectedToken", "expected value", self._peek().span)

    def _parse_array(self, opening: Token) -> AstArray:
        items: list[AstArrayItem] = []
        self._skip_trivia()
        while not self._check("]") and not self._check("eof"):
            start = self._peek().span.start
            if self._match("..."):
                sub = self._parse_substitution()
                items.append(AstArraySpread(sub, Span(start, sub.span.end)))
            else:
                value = self._parse_value()
                items.append(AstArrayValue(value, value.span))
            self._skip_trivia()
            if not self._match(","):
                break
            self._skip_trivia()
            if self._check(","):
                raise SconError("UnexpectedToken", "consecutive commas are invalid", self._peek().span)
        closing = self._expect("]", "expected ']'")
        return AstArray(items, Span(opening.span.start, closing.span.end))

    def _parse_substitution(self) -> AstSubstitution:
        start = self._expect("subst", "expected '${'")
        path = self._parse_path()
        end = self._expect("}", "expected '}'")
        return AstSubstitution(path, Span(start.span.start, end.span.end))

    def _parse_path(self) -> AstPath:
        first = self._parse_path_segment()
        segments = [first]
        while self._match("."):
            segments.append(self._parse_path_segment())
        return AstPath(segments, Span(first.span.start, segments[-1].span.end))

    def _parse_path_segment(self) -> AstPathSegment:
        if self._match("identifier"):
            token = self._previous()
            return AstPathSegment(token.text, False, token.span)
        if self._check("string"):
            string = self._parse_string()
            return AstPathSegment(string.value, True, string.span)
        raise SconError("UnexpectedToken", "expected path segment", self._peek().span)

    def _parse_string(self) -> AstString:
        token = self._expect("string", "expected string")
        parts, value = _parse_string_parts(token)
        return AstString(value, token.text, parts, token.span)

    def _skip_trivia(self) -> None:
        while self._match("ws") or self._match("newline") or self._match("comment"):
            pass

    def _skip_inline_trivia(self) -> None:
        while self._match("ws") or self._match("comment"):
            pass

    def _match(self, kind: str) -> bool:
        if not self._check(kind):
            return False
        self.index += 1
        return True

    def _check(self, kind: str) -> bool:
        return self._peek().kind == kind

    def _expect(self, kind: str, message: str) -> Token:
        if self._check(kind):
            self.index += 1
            return self._previous()
        raise SconError("UnexpectedToken", message, self._peek().span)

    def _peek(self) -> Token:
        return self.tokens[min(self.index, len(self.tokens) - 1)]

    def _previous(self) -> Token:
        return self.tokens[self.index - 1]


def _parse_string_parts(token: Token) -> tuple[list[StringPart], str]:
    raw = token.text
    parts: list[StringPart] = []
    out = ""
    value = ""
    index = 1
    while index < len(raw) - 1:
        ch = raw[index]
        index += 1
        if ch == "$" and index < len(raw) and raw[index] == "{":
            if out:
                parts.append(StringLiteralPart(out))
                value += out
                out = ""
            path_start = index + 1
            close = raw.find("}", path_start)
            if close < 0:
                raise SconError("UnterminatedString", "unterminated interpolation", token.span)
            parts.append(StringInterpolationPart(
                _parse_interpolation_path(raw[path_start:close], token.span.start + path_start),
                Span(token.span.start + index - 1, token.span.start + close + 1),
            ))
            index = close + 1
            continue
        if ch != "\\":
            out += ch
            continue
        escaped = raw[index]
        index += 1
        escapes = {'"': '"', "\\": "\\", "/": "/", "b": "\b", "f": "\f", "n": "\n", "r": "\r", "t": "\t", "$": "$"}
        if escaped in escapes:
            out += escapes[escaped]
        elif escaped == "u":
            out += chr(int(raw[index:index + 4], 16))
            index += 4
        else:
            raise SconError("InvalidEscape", "invalid string escape", token.span)
    if out or not parts:
        parts.append(StringLiteralPart(out))
        value += out
    return parts, value


def _parse_interpolation_path(text: str, base: int) -> AstPath:
    tokens = lex(text)
    parser = Parser([
        Token(token.kind, token.text, Span(token.span.start + base, token.span.end + base))
        for token in tokens
    ])
    path = parser._parse_path()
    parser._expect("eof", "expected end of interpolation")
    return path
