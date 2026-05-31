from __future__ import annotations

from dataclasses import dataclass

from .error import Span, fail


@dataclass(frozen=True)
class Token:
    kind: str
    text: str
    span: Span


def lex(source: str) -> list[Token]:
    tokens: list[Token] = []
    index = 0

    def add(kind: str, start: int, end: int) -> None:
        tokens.append(Token(kind, source[start:end], Span(start, end)))

    while index < len(source):
        start = index
        ch = source[index]
        if ch in " \t":
            while index < len(source) and source[index] in " \t":
                index += 1
            add("ws", start, index)
        elif ch == "\n":
            index += 1
            add("newline", start, index)
        elif ch == "\r":
            if index + 1 >= len(source) or source[index + 1] != "\n":
                fail("InvalidCharacter", "standalone CR is invalid", start, start + 1)
            index += 2
            add("newline", start, index)
        elif ch == "#" or (ch == "/" and index + 1 < len(source) and source[index + 1] == "/"):
            index += 1 if ch == "#" else 2
            while index < len(source) and source[index] not in "\n\r":
                index += 1
            add("comment", start, index)
        elif ch == '"':
            index = _lex_string(source, index)
            add("string", start, index)
        elif ch == "$":
            if index + 1 >= len(source) or source[index + 1] != "{":
                fail("InvalidCharacter", "unexpected character '$'", start, start + 1)
            index += 2
            add("subst", start, index)
        elif ch in "{}[]=,":
            index += 1
            add(ch, start, index)
        elif ch == ".":
            if source[index:index + 3] == "...":
                index += 3
                add("...", start, index)
            else:
                index += 1
                add(".", start, index)
        elif ch == "-":
            if index + 1 >= len(source) or not source[index + 1].isdigit():
                fail("UnexpectedToken", "expected digit after '-'", start, start + 1)
            index = _lex_number(source, index)
            add("number", start, index)
        elif ch in "?:":
            fail("UnexpectedToken", "unexpected character", start, start + 1)
        elif ch.isdigit():
            index = _lex_number(source, index)
            add("number", start, index)
        elif _is_identifier_start(ch):
            while index < len(source) and _is_identifier_part(source[index]):
                index += 1
            text = source[start:index]
            add(text if text in {"true", "false", "null", "include"} else "identifier", start, index)
        elif ch.isspace():
            fail("InvalidWhitespace", "invalid whitespace outside strings", start, start + 1)
        else:
            fail("InvalidCharacter", "unexpected character", start, start + 1)

    tokens.append(Token("eof", "", Span(len(source), len(source))))
    return tokens


def _lex_string(source: str, index: int) -> int:
    start = index
    index += 1
    while index < len(source):
        ch = source[index]
        index += 1
        if ch == '"':
            return index
        if ch in "\n\r":
            fail("UnterminatedString", "raw multiline strings are invalid", index - 1, index)
        if ch == "\\":
            if index >= len(source):
                fail("UnterminatedString", "unterminated string escape", index, index)
            escaped = source[index]
            index += 1
            if escaped in '"\\/bfnrt$':
                continue
            if escaped == "u":
                for _ in range(4):
                    if index >= len(source) or source[index] not in "0123456789abcdefABCDEF":
                        fail("InvalidEscape", "invalid unicode escape", index, min(index + 1, len(source)))
                    index += 1
                continue
            fail("InvalidEscape", "invalid string escape", index - 2, index - 1)
    fail("UnterminatedString", "unterminated string", start, len(source))
    raise AssertionError


def _lex_number(source: str, index: int) -> int:
    start = index
    if source[index] == "-":
        index += 1
    if index < len(source) and source[index] == "0":
        index += 1
        if index < len(source) and source[index].isdigit():
            fail("InvalidNumber", "leading zeroes are invalid", start, index)
    else:
        if index >= len(source) or source[index] not in "123456789":
            fail("InvalidNumber", "invalid number", start, index)
        while index < len(source) and source[index].isdigit():
            index += 1
    if index < len(source) and source[index] == ".":
        index += 1
        if index >= len(source) or not source[index].isdigit():
            fail("InvalidNumber", "expected digit after decimal point", start, index)
        while index < len(source) and source[index].isdigit():
            index += 1
    if index < len(source) and source[index] in "eE":
        index += 1
        if index < len(source) and source[index] in "+-":
            index += 1
        if index >= len(source) or not source[index].isdigit():
            fail("InvalidNumber", "expected exponent digit", start, index)
        while index < len(source) and source[index].isdigit():
            index += 1
    return index


def _is_identifier_start(ch: str) -> bool:
    return ch == "_" or "A" <= ch <= "Z" or "a" <= ch <= "z"


def _is_identifier_part(ch: str) -> bool:
    return _is_identifier_start(ch) or ch.isdigit() or ch == "-"
