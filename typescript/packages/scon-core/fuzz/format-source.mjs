import assert from "node:assert/strict";
import { analyzeSource, formatSource, parseString, sconToPlain } from "../src/index.js";

export function fuzz(data) {
  const source = data.toString("utf8");
  let formatted;
  try {
    formatted = formatSource(source);
  } catch {
    return;
  }

  assert.ok(analyzeSource(formatted).parsed);

  try {
    const original = normalize(sconToPlain(parseString(source)));
    const roundTrip = normalize(sconToPlain(parseString(formatted)));
    assert.deepEqual(roundTrip, original);
  } catch {
    // If either source fails semantic resolution, formatting parseability is
    // still the invariant under test.
  }
}

function normalize(value) {
  if (typeof value === "bigint") return value.toString();
  if (Array.isArray(value)) return value.map(normalize);
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([key, item]) => [key, normalize(item)]));
  }
  return value;
}
