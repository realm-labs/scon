import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import { analyzeSource, formatSource, formatValue, parseFile, parseString, SconError, sconToPlain } from "../src/index.js";

type Manifest = { cases: Case[] };
type Case = { id: string; description: string; entry: string; kind: "valid" | "invalid"; expected: string };

const root = join(import.meta.dirname, "../../../../tests/conformance");
const manifest = JSON.parse(readFileSync(join(root, "manifest.json"), "utf8")) as Manifest;

for (const entry of manifest.cases) {
  test(`conformance ${entry.id}`, () => {
    if (entry.kind === "valid") {
      const actual = normalizeNumbers(sconToPlain(parseFile(join(root, entry.entry))));
      const expected = normalizeNumbers(JSON.parse(readFileSync(join(root, entry.expected), "utf8")) as unknown);
      assert.deepEqual(actual, expected);
      parseString(formatValue(parseFile(join(root, entry.entry))));
      return;
    }
    assert.throws(
      () => parseFile(join(root, entry.entry)),
      (error) => {
        const expected = JSON.parse(readFileSync(join(root, entry.expected), "utf8")) as { code: string };
        return error instanceof SconError && error.code === expected.code;
      },
    );
  });
}

test("analysis and source formatter preserve source constructs", () => {
  const source = "defaults { port = 8080 }\nserver = ${defaults.port}\nitems = [1, ...${extra}]\n";
  const analysis = analyzeSource(source);
  assert.equal(analysis.diagnostics.length, 1);
  assert.equal(analysis.diagnostics[0]?.code, "MissingReference");
  assert.ok(analysis.symbols.length >= 3);
  assert.equal(analysis.references.length, 2);

  const formatted = formatSource("# keep me\ninclude \"base.scon\"\ndefaults { port = 8080 }\nserver = ${defaults.port}\nitems = [1, ...${extra}]\n");
  assert.ok(analyzeSource(formatted).parsed);
  assert.match(formatted, /# keep me/);
  assert.match(formatted, /include "base\.scon"/);
  assert.match(formatted, /\.\.\.\$\{extra\}/);
});

function normalizeNumbers(value: unknown): unknown {
  if (typeof value === "number") return Number(value);
  if (typeof value === "string" && /^-?\d+(\.\d+)?([eE][+-]?\d+)?$/.test(value)) return Number(value);
  if (Array.isArray(value)) return value.map(normalizeNumbers);
  if (value && typeof value === "object") return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, normalizeNumbers(v)]));
  return value;
}
