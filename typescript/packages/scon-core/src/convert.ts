import { SconError } from "./error.js";
import type { SconValue } from "./value.js";

export function sconToPlain(value: SconValue): unknown {
  if (value === null || typeof value === "boolean" || typeof value === "string") return value;
  if (Array.isArray(value)) return value.map(sconToPlain);
  if (value instanceof Map) {
    return Object.fromEntries([...value.entries()].map(([key, item]) => [key, sconToPlain(item)]));
  }
  if (value.kind === "f64") return value.value;
  const asNumber = Number(value.value);
  return BigInt(asNumber) === value.value && Number.isSafeInteger(asNumber)
    ? asNumber
    : value.value.toString();
}

export function plainToScon(value: unknown): SconValue {
  if (value === null) return null;
  if (typeof value === "boolean" || typeof value === "string") return value;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new SconError("Serde", "non-finite numbers cannot be serialized");
    return Number.isInteger(value)
      ? { kind: value < 0 ? "i64" : "u64", value: BigInt(value) }
      : { kind: "f64", value };
  }
  if (typeof value === "bigint") return { kind: value < 0n ? "i64" : "u64", value };
  if (Array.isArray(value)) return value.map(plainToScon);
  if (typeof value === "object") {
    const out = new Map<string, SconValue>();
    for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
      if (item === undefined || typeof item === "function" || typeof item === "symbol") {
        throw new SconError("Serde", `unsupported value for key ${key}`);
      }
      out.set(key, plainToScon(item));
    }
    return out;
  }
  throw new SconError("Serde", `unsupported value type ${typeof value}`);
}
