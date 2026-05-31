export type SconNumber =
  | { kind: "i64"; value: bigint }
  | { kind: "u64"; value: bigint }
  | { kind: "f64"; value: number };

export type SconObject = Map<string, SconValue>;

export type SconValue =
  | null
  | boolean
  | SconNumber
  | string
  | SconValue[]
  | SconObject;

export type ResolveOptions = {
  includeRoot?: string;
  maxFileSize?: number;
  maxIncludeDepth?: number;
  maxIncludeFiles?: number;
};

export function isSconNumber(value: unknown): value is SconNumber {
  return typeof value === "object" && value !== null && "kind" in value;
}

export function cloneSconValue(value: SconValue): SconValue {
  if (Array.isArray(value)) return value.map(cloneSconValue);
  if (value instanceof Map) {
    return new Map([...value.entries()].map(([key, item]) => [key, cloneSconValue(item)]));
  }
  return value;
}
