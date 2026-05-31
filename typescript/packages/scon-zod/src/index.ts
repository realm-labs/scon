import { formatValue, parseString, plainToScon, sconToPlain, SconError, type SconValue } from "@realmlabs/scon-core";
import type { z } from "zod";

export function decodeScon<T>(source: string, schema: z.ZodType<T>): T {
  return schema.parse(sconToPlain(parseString(source)));
}

export function encodeScon<T>(value: T, schema?: z.ZodType<T>): string {
  const parsed = schema ? schema.parse(value) : value;
  const sconValue = plainToScon(parsed);
  if (!(sconValue instanceof Map)) {
    throw new SconError("InvalidRootType", "SCON document root must be an object");
  }
  return formatValue(sconValue as SconValue);
}
