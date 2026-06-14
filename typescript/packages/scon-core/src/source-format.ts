import type {
  AstArray,
  AstMember,
  AstObject,
  AstPath,
  AstSubstitution,
  AstValue,
} from "./ast.js";
import { lex } from "./lexer.js";
import { parseDocument } from "./parser.js";

export function formatSource(source: string): string {
  const document = parseDocument(source);
  const comments = lex(source).filter((token) => token.kind === "comment").map((token) => `${token.text}\n`).join("");
  return comments + formatObjectBody(document.root, 0) + "\n";
}

function formatObjectBody(object: AstObject, indent: number): string {
  return object.members.map((member) => `${" ".repeat(indent)}${formatMember(member, indent)}\n`).join("");
}

function formatMember(member: AstMember, indent: number): string {
  switch (member.type) {
    case "include":
      return `include ${member.path.raw}`;
    case "objectSpread":
      return `...${formatSubstitution(member.sub)}`;
    case "field":
      return `${formatPath(member.path)} = ${formatValue(member.value, indent)}`;
  }
}

function formatValue(value: AstValue, indent: number): string {
  switch (value.type) {
    case "null":
      return "null";
    case "bool":
      return value.value ? "true" : "false";
    case "number":
      return value.raw;
    case "string":
      return value.raw;
    case "substitution":
      return formatSubstitution(value);
    case "array":
      return formatArray(value, indent);
    case "object":
      return value.object.members.length === 0 ? "{}" : `{\n${formatObjectBody(value.object, indent + 2)}${" ".repeat(indent)}}`;
  }
}

function formatArray(array: AstArray, indent: number): string {
  if (array.items.length === 0) return "[]";
  return `[\n${array.items.map((item) => {
    const content = item.type === "spread" ? `...${formatSubstitution(item.sub)}` : formatValue(item.value, indent + 2);
    return `${" ".repeat(indent + 2)}${content},\n`;
  }).join("")}${" ".repeat(indent)}]`;
}

function formatSubstitution(substitution: AstSubstitution): string {
  return `\${${formatPath(substitution.path)}}`;
}

function formatPath(path: AstPath): string {
  return path.segments.map((segment) => segment.quoted || !isUnquotedSegment(segment.value) ? quote(segment.value) : segment.value).join(".");
}

function isUnquotedSegment(value: string): boolean {
  return !isReservedSegment(value) && /^[A-Za-z_][A-Za-z0-9_-]*$/.test(value);
}

function quote(value: string): string {
  let out = "\"";
  for (const ch of value) {
    const code = ch.charCodeAt(0);
    if (ch === '"') out += "\\\"";
    else if (ch === "\\") out += "\\\\";
    else if (ch === "\n") out += "\\n";
    else if (ch === "\r") out += "\\r";
    else if (ch === "\t") out += "\\t";
    else if (ch === "\b") out += "\\b";
    else if (ch === "\f") out += "\\f";
    else if (code < 0x20) out += `\\u${code.toString(16).padStart(4, "0").toUpperCase()}`;
    else out += ch;
  }
  return `${out}"`;
}

function isReservedSegment(value: string): boolean {
  return value === "include" || value === "true" || value === "false" || value === "null";
}
