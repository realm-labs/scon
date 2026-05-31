import { fail } from "./error.js";
import type { Span } from "./error.js";

export type TokenKind =
  | "identifier"
  | "string"
  | "number"
  | "true"
  | "false"
  | "null"
  | "include"
  | "subst"
  | "{"
  | "}"
  | "["
  | "]"
  | "="
  | "."
  | ","
  | "..."
  | "comment"
  | "newline"
  | "ws"
  | "eof";

export type Token = {
  kind: TokenKind;
  text: string;
  span: Span;
};

export function lex(source: string): Token[] {
  const tokens: Token[] = [];
  let index = 0;
  const add = (kind: TokenKind, start: number, end: number) => {
    tokens.push({ kind, text: source.slice(start, end), span: { start, end } });
  };

  while (index < source.length) {
    const start = index;
    const ch = source[index]!;
    if (ch === " " || ch === "\t") {
      while (source[index] === " " || source[index] === "\t") index++;
      add("ws", start, index);
    } else if (ch === "\n") {
      index++;
      add("newline", start, index);
    } else if (ch === "\r") {
      if (source[index + 1] !== "\n") fail("InvalidCharacter", "standalone CR is invalid", start, start + 1);
      index += 2;
      add("newline", start, index);
    } else if (ch === "#" || (ch === "/" && source[index + 1] === "/")) {
      index += ch === "#" ? 1 : 2;
      while (index < source.length && source[index] !== "\n" && source[index] !== "\r") index++;
      add("comment", start, index);
    } else if (ch === '"') {
      index = lexString(source, index);
      add("string", start, index);
    } else if (ch === "$") {
      if (source[index + 1] !== "{") fail("InvalidCharacter", "unexpected character '$'", start, start + 1);
      index += 2;
      add("subst", start, index);
    } else if ("{}[]=,".includes(ch)) {
      index++;
      add(ch as TokenKind, start, index);
    } else if (ch === ".") {
      if (source.slice(index, index + 3) === "...") {
        index += 3;
        add("...", start, index);
      } else {
        index++;
        add(".", start, index);
      }
    } else if (ch === "-") {
      if (!isDigit(source[index + 1])) fail("UnexpectedToken", "expected digit after '-'", start, start + 1);
      index = lexNumber(source, index);
      add("number", start, index);
    } else if (ch === "?" || ch === ":") {
      fail("UnexpectedToken", "unexpected character", start, start + 1);
    } else if (isDigit(ch)) {
      index = lexNumber(source, index);
      add("number", start, index);
    } else if (isIdentifierStart(ch)) {
      while (isIdentifierPart(source[index] ?? "")) index++;
      const text = source.slice(start, index);
      add(text === "true" || text === "false" || text === "null" || text === "include" ? text : "identifier", start, index);
    } else if (/\s/u.test(ch)) {
      fail("InvalidWhitespace", "invalid whitespace outside strings", start, start + ch.length);
    } else {
      fail("InvalidCharacter", "unexpected character", start, start + ch.length);
    }
  }

  tokens.push({ kind: "eof", text: "", span: { start: source.length, end: source.length } });
  return tokens;
}

function lexString(source: string, index: number): number {
  const start = index++;
  while (index < source.length) {
    const ch = source[index++]!;
    if (ch === '"') return index;
    if (ch === "\n" || ch === "\r") fail("UnterminatedString", "raw multiline strings are invalid", index - 1, index);
    if (ch === "\\") {
      if (index >= source.length) fail("UnterminatedString", "unterminated string escape", index, index);
      const escaped = source[index++]!;
      if ('"\\/bfnrt$'.includes(escaped)) continue;
      if (escaped === "u") {
        for (let n = 0; n < 4; n++, index++) {
          if (!/[0-9a-fA-F]/.test(source[index] ?? "")) fail("InvalidEscape", "invalid unicode escape", index, index + 1);
        }
        continue;
      }
      fail("InvalidEscape", "invalid string escape", index - 2, index - 1);
    }
  }
  fail("UnterminatedString", "unterminated string", start, source.length);
}

function lexNumber(source: string, index: number): number {
  const start = index;
  if (source[index] === "-") index++;
  if (source[index] === "0") {
    index++;
    if (isDigit(source[index])) fail("InvalidNumber", "leading zeroes are invalid", start, index);
  } else {
    if (!/[1-9]/.test(source[index] ?? "")) fail("InvalidNumber", "invalid number", start, index);
    while (isDigit(source[index])) index++;
  }
  if (source[index] === ".") {
    index++;
    if (!isDigit(source[index])) fail("InvalidNumber", "expected digit after decimal point", start, index);
    while (isDigit(source[index])) index++;
  }
  if (source[index] === "e" || source[index] === "E") {
    index++;
    if (source[index] === "+" || source[index] === "-") index++;
    if (!isDigit(source[index])) fail("InvalidNumber", "expected exponent digit", start, index);
    while (isDigit(source[index])) index++;
  }
  return index;
}

function isDigit(ch: string | undefined): boolean {
  return !!ch && ch >= "0" && ch <= "9";
}

function isIdentifierStart(ch: string): boolean {
  return /^[A-Za-z_]$/.test(ch);
}

function isIdentifierPart(ch: string): boolean {
  return /^[A-Za-z0-9_-]$/.test(ch);
}
