import { parseString } from "../src/index.js";

export function fuzz(data) {
  const source = data.toString("utf8");
  try {
    parseString(source);
  } catch {
    // Expected syntax and semantic errors are not fuzz failures.
  }
}
