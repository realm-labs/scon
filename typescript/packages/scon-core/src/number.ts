import { SconError, type Span } from "./error.js";
import type { SconNumber } from "./value.js";

export function parseNumber(raw: string, span: Span): SconNumber {
  try {
    if (/[.eE]/.test(raw)) {
      const value = Number(raw);
      if (!Number.isFinite(value)) {
        throw invalidNumber(raw, span, "float value must be finite");
      }
      return { kind: "f64", value };
    }
    const value = BigInt(raw);
    if (raw.startsWith("-")) {
      if (value < -(1n << 63n)) {
        throw invalidNumber(raw, span, "signed integer is below i64 minimum");
      }
      return { kind: "i64", value };
    }
    if (value > (1n << 64n) - 1n) {
      throw invalidNumber(raw, span, "unsigned integer exceeds u64 maximum");
    }
    return { kind: "u64", value };
  } catch (error) {
    if (error instanceof SconError) throw error;
    throw invalidNumber(raw, span, "number literal cannot be parsed");
  }
}

export function numberToString(value: SconNumber): string {
  return value.kind === "f64" ? String(value.value) : value.value.toString();
}

function invalidNumber(raw: string, span: Span, reason: string): SconError {
  return new SconError("InvalidNumber", `invalid SCON number ${JSON.stringify(raw)}: ${reason}`, span);
}
