import assert from "node:assert/strict";
import test from "node:test";
import { z } from "zod";
import { decodeScon, encodeScon } from "../src/index.js";

const schema = z.object({
  name: z.string(),
  enabled: z.boolean(),
  port: z.number().int(),
  tags: z.array(z.string()),
  server: z.object({
    host: z.string(),
  }),
});

test("decodes SCON with Zod", () => {
  const value = decodeScon(
    `
    name = "demo"
    enabled = true
    port = 8080
    tags = ["api", "prod"]
    server.host = "127.0.0.1"
    `,
    schema,
  );
  assert.equal(value.server.host, "127.0.0.1");
});

test("encodes and decodes with Zod", () => {
  const value = {
    name: "demo",
    enabled: true,
    port: 8080,
    tags: ["api"],
    server: { host: "127.0.0.1" },
  };
  const source = encodeScon(value, schema);
  assert.deepEqual(decodeScon(source, schema), value);
});
