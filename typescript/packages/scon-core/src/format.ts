import { SconError } from "./error.js";
import { numberToString } from "./number.js";
import type { SconObject, SconValue } from "./value.js";

export function formatValue(value: SconValue): string {
  if (!(value instanceof Map)) throw new SconError("InvalidRootType", "SCON document root must be an object");
  return `${formatObjectBody(value, 0)}\n`;
}

function formatObjectBody(object: SconObject, indent: number): string {
  let out = "";
  for (const [key, value] of object) {
    out += `${" ".repeat(indent)}${formatKey(key)} = ${formatScon(value, indent)}\n`;
  }
  return out;
}

function formatScon(value: SconValue, indent: number): string {
  if (value === null) return "null";
  if (typeof value === "boolean") return String(value);
  if (typeof value === "string") return quote(value, true);
  if (Array.isArray(value)) {
    if (value.length === 0) return "[]";
    return `[\n${value.map((item) => `${" ".repeat(indent + 2)}${formatScon(item, indent + 2)},\n`).join("")}${" ".repeat(indent)}]`;
  }
  if (value instanceof Map) {
    if (value.size === 0) return "{}";
    return `{\n${formatObjectBody(value, indent + 2)}${" ".repeat(indent)}}`;
  }
  return numberToString(value);
}

function formatKey(key: string): string {
  return isUnquotedKey(key) ? key : quote(key);
}

function quote(value: string, escapeInterpolation = false): string {
  let out = "\"";
  for (const ch of value) {
    const code = ch.charCodeAt(0);
    if (ch === '"') out += '\\"';
    else if (ch === "\\") out += "\\\\";
    else if (ch === "\n") out += "\\n";
    else if (ch === "\r") out += "\\r";
    else if (ch === "\t") out += "\\t";
    else if (ch === "\b") out += "\\b";
    else if (ch === "\f") out += "\\f";
    else if (ch === "$" && escapeInterpolation) out += "\\$";
    else if (code < 0x20) out += `\\u${code.toString(16).padStart(4, "0").toUpperCase()}`;
    else out += ch;
  }
  return `${out}"`;
}

function isUnquotedKey(key: string): boolean {
  return !isReservedSegment(key) && /^[A-Za-z_][A-Za-z0-9_-]*$/.test(key);
}

function isReservedSegment(value: string): boolean {
  return value === "include" || value === "true" || value === "false" || value === "null";
}
