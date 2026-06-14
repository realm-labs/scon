from __future__ import annotations

from io import StringIO

from .format import _quote
from .lexer import lex
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
    AstPath,
    AstString,
    AstSubstitution,
    AstValue,
    parse_document,
)


def format_source(source: str) -> str:
    document = parse_document(source)
    out = StringIO()
    for token in lex(source):
        if token.kind == "comment":
            out.write(token.text)
            out.write("\n")
    _write_object_body(out, document.root, 0)
    out.write("\n")
    return out.getvalue()


def _write_object_body(out: StringIO, obj: AstObject, indent: int) -> None:
    for member in obj.members:
        out.write(" " * indent)
        if isinstance(member, AstInclude):
            out.write(f"include {member.path.raw}")
        elif isinstance(member, AstObjectSpread):
            out.write("...")
            _write_substitution(out, member.sub)
        elif isinstance(member, AstField):
            _write_path(out, member.path)
            out.write(" = ")
            _write_value(out, member.value, indent)
        out.write("\n")


def _write_value(out: StringIO, value: AstValue, indent: int) -> None:
    if isinstance(value, AstNull):
        out.write("null")
    elif isinstance(value, AstBool):
        out.write("true" if value.value else "false")
    elif isinstance(value, AstNumber):
        out.write(value.raw)
    elif isinstance(value, AstString):
        out.write(value.raw)
    elif isinstance(value, AstSubstitution):
        _write_substitution(out, value)
    elif isinstance(value, AstArray):
        if not value.items:
            out.write("[]")
            return
        out.write("[\n")
        for item in value.items:
            out.write(" " * (indent + 2))
            if isinstance(item, AstArraySpread):
                out.write("...")
                _write_substitution(out, item.sub)
            elif isinstance(item, AstArrayValue):
                _write_value(out, item.value, indent + 2)
            out.write(",\n")
        out.write(" " * indent)
        out.write("]")
    elif isinstance(value, AstObjectValue):
        if not value.object.members:
            out.write("{}")
            return
        out.write("{\n")
        _write_object_body(out, value.object, indent + 2)
        out.write(" " * indent)
        out.write("}")


def _write_substitution(out: StringIO, value: AstSubstitution) -> None:
    out.write("${")
    _write_path(out, value.path)
    out.write("}")


def _write_path(out: StringIO, path: AstPath) -> None:
    for index, segment in enumerate(path.segments):
        if index:
            out.write(".")
        if segment.quoted:
            out.write(_quote(segment.value, escape_interpolation=False))
        else:
            out.write(segment.value)
