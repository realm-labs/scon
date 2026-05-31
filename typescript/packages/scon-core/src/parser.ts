import type {
  AstArray,
  AstArrayItem,
  AstMember,
  AstObject,
  AstPath,
  AstPathSegment,
  AstString,
  AstSubstitution,
  AstValue,
  Document,
  StringPart,
} from "./ast.js";
import { fail, SconError } from "./error.js";
import { lex, type Token, type TokenKind } from "./lexer.js";

export function parseDocument(source: string, file?: string): Document {
  return { root: new Parser(lex(source)).parse(), file };
}

class Parser {
  private index = 0;

  constructor(private readonly tokens: Token[]) {}

  parse(): AstObject {
    this.skipTrivia();
    const root = this.match("{")
      ? this.parseObject(this.previous())
      : this.check("[")
        ? this.invalidRoot()
        : this.parseObjectBody(this.peek().span.start);
    this.skipTrivia();
    this.expect("eof", "expected end of file");
    return root;
  }

  parsePath(): AstPath {
    const first = this.parsePathSegment();
    const segments = [first];
    while (this.match(".")) segments.push(this.parsePathSegment());
    return { segments, span: { start: first.span.start, end: segments.at(-1)!.span.end } };
  }

  private parseObject(opening: Token): AstObject {
    const members = this.parseMembers("}");
    const closing = this.expect("}", "expected '}'");
    return { members, span: { start: opening.span.start, end: closing.span.end } };
  }

  private parseObjectBody(start: number): AstObject {
    const members = this.parseMembers("eof");
    return { members, span: { start, end: members.at(-1)?.span.end ?? start } };
  }

  private parseMembers(end: TokenKind): AstMember[] {
    const members: AstMember[] = [];
    this.skipTrivia();
    while (!this.check(end) && !this.check("eof")) {
      members.push(this.parseMember());
      this.skipTrivia();
      if (this.match(",")) {
        this.skipTrivia();
        if (this.check(",")) throw new SconError("UnexpectedToken", "consecutive commas are invalid", this.peek().span);
      }
    }
    return members;
  }

  private parseMember(): AstMember {
    this.skipTrivia();
    if (this.match("include")) {
      const include = this.previous();
      this.skipInlineTrivia();
      const path = this.parseString();
      if (path.parts.some((part) => part.type === "interpolation")) {
        throw new SconError("UnexpectedToken", "include path must be a literal string", path.span);
      }
      return { type: "include", path, span: { start: include.span.start, end: path.span.end } };
    }
    if (this.match("...")) {
      const spread = this.previous();
      const sub = this.parseSubstitution();
      return { type: "objectSpread", sub, span: { start: spread.span.start, end: sub.span.end } };
    }
    const path = this.parsePath();
    this.skipInlineTrivia();
    let value: AstValue;
    if (this.match("=")) {
      this.skipInlineTrivia();
      if (this.check("newline")) throw new SconError("UnexpectedToken", "field value cannot start on the next line", this.peek().span);
      value = this.parseValue();
    } else if (this.match("{")) {
      const object = this.parseObject(this.previous());
      value = { type: "object", object, span: object.span };
    } else {
      throw new SconError("UnexpectedToken", "expected '=' or object shorthand", this.peek().span);
    }
    return { type: "field", path, value, span: { start: path.span.start, end: value.span.end } };
  }

  private parseValue(): AstValue {
    this.skipTrivia();
    if (this.match("null")) return { type: "null", span: this.previous().span };
    if (this.match("true")) return { type: "bool", value: true, span: this.previous().span };
    if (this.match("false")) return { type: "bool", value: false, span: this.previous().span };
    if (this.match("number")) {
      const token = this.previous();
      return { type: "number", raw: token.text, span: token.span };
    }
    if (this.check("string")) return this.parseString();
    if (this.match("{")) {
      const object = this.parseObject(this.previous());
      return { type: "object", object, span: object.span };
    }
    if (this.match("[")) return this.parseArray(this.previous());
    if (this.check("subst")) return this.parseSubstitution();
    throw new SconError("UnexpectedToken", "expected value", this.peek().span);
  }

  private parseArray(opening: Token): AstArray {
    const items: AstArrayItem[] = [];
    this.skipTrivia();
    while (!this.check("]") && !this.check("eof")) {
      const start = this.peek().span.start;
      if (this.match("...")) {
        const sub = this.parseSubstitution();
        items.push({ type: "spread", sub, span: { start, end: sub.span.end } });
      } else {
        const value = this.parseValue();
        items.push({ type: "value", value, span: value.span });
      }
      this.skipTrivia();
      if (!this.match(",")) break;
      this.skipTrivia();
      if (this.check(",")) throw new SconError("UnexpectedToken", "consecutive commas are invalid", this.peek().span);
    }
    const closing = this.expect("]", "expected ']'");
    return { type: "array", items, span: { start: opening.span.start, end: closing.span.end } };
  }

  private parseSubstitution(): AstSubstitution {
    const start = this.expect("subst", "expected '${'");
    const path = this.parsePath();
    const end = this.expect("}", "expected '}'");
    return { type: "substitution", path, span: { start: start.span.start, end: end.span.end } };
  }

  private parsePathSegment(): AstPathSegment {
    if (this.match("identifier")) {
      const token = this.previous();
      return { value: token.text, quoted: false, span: token.span };
    }
    if (this.check("string")) {
      const string = this.parseString();
      return { value: string.value, quoted: true, span: string.span };
    }
    throw new SconError("UnexpectedToken", "expected path segment", this.peek().span);
  }

  private parseString(): AstString {
    const token = this.expect("string", "expected string");
    const { parts, value } = parseStringParts(token);
    return { type: "string", value, raw: token.text, parts, span: token.span };
  }

  private skipTrivia(): void {
    while (this.match("ws") || this.match("newline") || this.match("comment")) {}
  }

  private skipInlineTrivia(): void {
    while (this.match("ws") || this.match("comment")) {}
  }

  private match(kind: TokenKind): boolean {
    if (!this.check(kind)) return false;
    this.index++;
    return true;
  }

  private check(kind: TokenKind): boolean {
    return this.peek().kind === kind;
  }

  private expect(kind: TokenKind, message: string): Token {
    if (this.check(kind)) {
      this.index++;
      return this.previous();
    }
    throw new SconError("UnexpectedToken", message, this.peek().span);
  }

  private peek(): Token {
    return this.tokens[this.index] ?? this.tokens[this.tokens.length - 1]!;
  }

  private previous(): Token {
    return this.tokens[this.index - 1]!;
  }

  private invalidRoot(): never {
    throw new SconError("InvalidRootType", "SCON document root must be an object", this.peek().span);
  }
}

function parseStringParts(token: Token): { parts: StringPart[]; value: string } {
  const raw = token.text;
  const parts: StringPart[] = [];
  let out = "";
  let value = "";
  for (let index = 1; index < raw.length - 1;) {
    const ch = raw[index++]!;
    if (ch === "$" && raw[index] === "{") {
      if (out) {
        parts.push({ type: "literal", value: out });
        value += out;
        out = "";
      }
      const pathStart = index + 1;
      const close = raw.indexOf("}", pathStart);
      if (close < 0) throw new SconError("UnterminatedString", "unterminated interpolation", token.span);
      parts.push({
        type: "interpolation",
        path: parseInterpolationPath(raw.slice(pathStart, close), token.span.start + pathStart),
        span: { start: token.span.start + index - 1, end: token.span.start + close + 1 },
      });
      index = close + 1;
      continue;
    }
    if (ch !== "\\") {
      out += ch;
      continue;
    }
    const escaped = raw[index++]!;
    const escapes: Record<string, string> = {
      '"': '"',
      "\\": "\\",
      "/": "/",
      b: "\b",
      f: "\f",
      n: "\n",
      r: "\r",
      t: "\t",
      $: "$",
    };
    if (escaped in escapes) out += escapes[escaped];
    else if (escaped === "u") {
      out += String.fromCharCode(Number.parseInt(raw.slice(index, index + 4), 16));
      index += 4;
    } else {
      throw new SconError("InvalidEscape", "invalid string escape", token.span);
    }
  }
  if (out || parts.length === 0) {
    parts.push({ type: "literal", value: out });
    value += out;
  }
  return { parts, value };
}

function parseInterpolationPath(text: string, base: number): AstPath {
  if (text.startsWith(".") || text.startsWith("?") || text.includes(":-")) {
    fail("UnexpectedToken", "invalid substitution path", base, Math.max(base + 1, base + text.length));
  }
  const path = new Parser(lex(text)).parsePath();
  path.span = { start: path.span.start + base, end: path.span.end + base };
  for (const segment of path.segments) {
    segment.span = { start: segment.span.start + base, end: segment.span.end + base };
  }
  return path;
}
