import { readFileSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, normalize, resolve } from "node:path";

import type { AstArray, AstField, AstInclude, AstObject, AstPath, AstString, AstValue, Document } from "./ast.js";
import { SconError, type Span } from "./error.js";
import { numberToString, parseNumber } from "./number.js";
import { parseDocument } from "./parser.js";
import { cloneSconValue, isSconNumber, type ResolveOptions, type SconObject, type SconValue } from "./value.js";

export function parseString(source: string, options: ResolveOptions = {}): SconValue {
  return new Resolver(options).eval(parseDocument(source));
}

export function parseFile(path: string, options: ResolveOptions = {}): SconValue {
  const file = resolve(path);
  const includeRoot = options.includeRoot ? resolve(options.includeRoot) : dirname(file);
  const source = readFileSync(file, "utf8");
  if (source.length > limit(options.maxFileSize, 16 * 1024 * 1024)) {
    throw new SconError("ResourceLimitExceeded", "maximum file size exceeded");
  }
  const resolver = new Resolver({ ...options, includeRoot });
  resolver.stack.push(file);
  resolver.seen.add(file);
  return resolver.eval(parseDocument(source, file));
}

export function getPath(value: SconValue, path: string): SconValue {
  let current = value;
  for (const segment of path.split(".")) {
    if (!(current instanceof Map)) throw new SconError("TypeMismatch", "path segment requires object");
    const next = current.get(segment);
    if (next === undefined) throw new SconError("MissingReference", "path is not defined");
    current = next;
  }
  return current;
}

class Resolver {
  stack: string[] = [];
  seen = new Set<string>();
  private cache = new Map<string, Document>();
  private root = new EvalObject();
  private inProgress: string[][] = [[]];

  constructor(private readonly options: ResolveOptions) {}

  eval(document: Document): SconValue {
    this.evalObject(document.root, [], document.file);
    return publicObject(this.root);
  }

  private evalObject(object: AstObject, path: string[], file?: string): void {
    if (path.length > limit(this.options.maxIncludeDepth, 512)) {
      throw new SconError("ResourceLimitExceeded", "maximum object depth exceeded", object.span);
    }
    let localSeen = false;
    for (const member of object.members) {
      if (member.type === "objectSpread") {
        if (localSeen) throw new SconError("InvalidSpread", "object spread must appear before local members", member.span);
        const target = this.lookup(member.sub.path, member.span).value;
        if (!(target instanceof EvalObject)) throw new SconError("TypeMismatch", "object spread target is not an object", member.span);
        overlayBase(this.objectAt(path, member.span), target);
      } else if (member.type === "include") {
        const included = this.loadInclude(file, member);
        this.evalObject(included.root, path, included.file);
      } else {
        localSeen = true;
        this.evalField(member, path, file);
      }
    }
  }

  private evalField(field: AstField, current: string[], file?: string): void {
    const target = [...current, ...field.path.segments.map((segment) => segment.value)];
    if (field.value.type === "object") {
      this.ensureObject(target, field.span);
      this.inProgress.push(target);
      this.evalObject(field.value.object, target, file);
      this.inProgress.pop();
    } else {
      this.insert(target, this.evalValue(field.value, file), "ordinary", field.span);
    }
  }

  private evalValue(value: AstValue, file?: string): SconValue | EvalObject {
    switch (value.type) {
      case "null":
        return null;
      case "bool":
        return value.value;
      case "number":
        return parseNumber(value.raw, value.span);
      case "string":
        return this.evalString(value);
      case "substitution":
        return cloneValue(this.lookup(value.path, value.span).value);
      case "array":
        return this.evalArray(value, file);
      case "object": {
        const nested = new Resolver(this.options);
        nested.stack = this.stack;
        nested.seen = this.seen;
        nested.cache = this.cache;
        nested.evalObject(value.object, [], file);
        return nested.root;
      }
    }
  }

  private evalArray(value: AstArray, file?: string): SconValue[] {
    const out: SconValue[] = [];
    for (const item of value.items) {
      if (out.length >= 1_000_000) {
        throw new SconError("ResourceLimitExceeded", "maximum array length exceeded", item.span);
      }
      if (item.type === "value") {
        out.push(publicMaybe(this.evalValue(item.value, file)));
      } else {
        const target = this.lookup(item.sub.path, item.span).value;
        if (!Array.isArray(target)) throw new SconError("TypeMismatch", "array spread target is not an array", item.span);
        out.push(...target.map(cloneSconValue));
      }
    }
    return out;
  }

  private evalString(value: AstString): string {
    if (value.parts.length === 1 && value.parts[0]!.type === "literal") return value.parts[0].value;
    let out = "";
    for (const part of value.parts) {
      if (part.type === "literal") {
        out += part.value;
        continue;
      }
      const replacement = this.lookup(part.path, part.span).value;
      if (typeof replacement === "string") out += replacement;
      else if (typeof replacement === "boolean") out += String(replacement);
      else if (isSconNumber(replacement)) out += numberToString(replacement);
      else throw new SconError("TypeMismatch", "interpolation requires string, number, or boolean", part.span);
    }
    return out;
  }

  private lookup(path: AstPath, span: Span): EvalEntry {
    const names = path.segments.map((segment) => segment.value);
    if (this.inProgress.some((active) => samePath(active, names))) {
      throw new SconError("MissingReference", "reference is not completed yet", span);
    }
    let object = this.root;
    let entry: EvalEntry | undefined;
    for (let index = 0; index < names.length; index++) {
      entry = object.entries.get(names[index]!);
      if (!entry) throw new SconError("MissingReference", `missing reference '${names[index]}'`, span);
      if (index < names.length - 1) {
        if (!(entry.value instanceof EvalObject)) throw new SconError("TypeMismatch", "reference path crosses non-object value", span);
        object = entry.value;
      }
    }
    return entry!;
  }

  private ensureObject(path: string[], span: Span): void {
    let object = this.root;
    for (let index = 0; index < path.length; index++) {
      const name = path[index]!;
      let entry = object.entries.get(name);
      if (!entry) {
        const child = new EvalObject();
        entry = { value: child, layer: "local", kind: "structural" };
        object.set(name, entry);
        object = child;
        continue;
      }
      if (!(entry.value instanceof EvalObject)) throw new SconError("PathConflict", "path conflicts with scalar value", span);
      if (index === path.length - 1 && entry.layer === "local" && entry.kind !== "structural") {
        throw new SconError("PathConflict", "object field conflicts with ordinary value", span);
      }
      entry.layer = "local";
      entry.kind = "structural";
      object = entry.value;
    }
  }

  private insert(path: string[], value: SconValue | EvalObject, kind: Kind, span: Span): void {
    let object = this.root;
    for (const name of path.slice(0, -1)) {
      let entry = object.entries.get(name);
      if (!entry) {
        const child = new EvalObject();
        entry = { value: child, layer: "local", kind: "structural" };
        object.set(name, entry);
        object = child;
      } else {
        if (!(entry.value instanceof EvalObject)) throw new SconError("PathConflict", "path conflicts with scalar value", span);
        object = entry.value;
      }
    }
    const leaf = path.at(-1)!;
    const existing = object.entries.get(leaf);
    if (!existing) {
      object.set(leaf, { value, layer: "local", kind });
      return;
    }
    if (existing.layer === "base") {
      overlayLocal(existing, value, kind);
      return;
    }
    throw new SconError("DuplicateKey", `duplicate key '${leaf}'`, span);
  }

  private objectAt(path: string[], span: Span): EvalObject {
    let object = this.root;
    for (const name of path) {
      const entry = object.entries.get(name);
      if (!entry) throw new SconError("PathConflict", "target object does not exist", span);
      if (!(entry.value instanceof EvalObject)) throw new SconError("PathConflict", "target path is not an object", span);
      object = entry.value;
    }
    return object;
  }

  private loadInclude(file: string | undefined, include: AstInclude): Document {
    const path = include.path.value;
    if (invalidIncludePath(path)) throw new SconError("InvalidIncludePath", "invalid include path", include.span);
    if (escapesIncludeRoot(path)) throw new SconError("IncludePathDenied", "include path escapes include root", include.span);
    const root = resolve(this.options.includeRoot ?? (file ? dirname(file) : "."));
    const base = file ? dirname(file) : root;
    const candidate = normalize(resolve(join(base, path)));
    if (!withinRoot(candidate, root)) throw new SconError("IncludePathDenied", "include path escapes include root", include.span);
    if (this.stack.includes(candidate)) throw new SconError("IncludeCycle", `include cycle: ${candidate}`, include.span);
    if (this.stack.length >= limit(this.options.maxIncludeDepth, 64)) {
      throw new SconError("ResourceLimitExceeded", "maximum include depth exceeded", include.span);
    }
    this.seen.add(candidate);
    if (this.seen.size > limit(this.options.maxIncludeFiles, 1024)) {
      throw new SconError("ResourceLimitExceeded", "maximum include file count exceeded", include.span);
    }
    const cached = this.cache.get(candidate);
    if (cached) return cached;

    let stat;
    try {
      stat = statSync(candidate);
    } catch (error) {
      throw new SconError("IncludeNotFound", `include file not found: ${String(error)}`, include.span);
    }
    if (!stat.isFile()) throw new SconError("IncludeNotFile", "include path is not a file", include.span);
    if (stat.size > limit(this.options.maxFileSize, 16 * 1024 * 1024)) {
      throw new SconError("ResourceLimitExceeded", "maximum file size exceeded", include.span);
    }

    this.stack.push(candidate);
    try {
      const document = parseDocument(readFileSync(candidate, "utf8"), candidate);
      this.cache.set(candidate, document);
      return document;
    } catch (error) {
      if (error instanceof SconError) {
        throw new SconError(error.code === "InvalidRootType" ? "IncludeRootTypeError" : "IncludeParseError", error.message, error.span);
      }
      throw error;
    } finally {
      this.stack.pop();
    }
  }
}

type Layer = "base" | "local";
type Kind = "structural" | "ordinary";
type EvalEntry = { value: SconValue | EvalObject; layer: Layer; kind: Kind };

class EvalObject {
  entries = new Map<string, EvalEntry>();

  set(key: string, value: EvalEntry): void {
    this.entries.set(key, value);
  }
}

function publicObject(object: EvalObject): SconObject {
  const out = new Map<string, SconValue>();
  for (const [key, entry] of object.entries) out.set(key, publicMaybe(entry.value));
  return out;
}

function publicMaybe(value: SconValue | EvalObject): SconValue {
  return value instanceof EvalObject ? publicObject(value) : value;
}

function cloneValue(value: SconValue | EvalObject): SconValue | EvalObject {
  if (value instanceof EvalObject) {
    const out = new EvalObject();
    for (const [key, entry] of value.entries) {
      out.set(key, { value: cloneValue(entry.value), layer: entry.layer, kind: entry.kind });
    }
    return out;
  }
  return cloneSconValue(value);
}

function overlayBase(target: EvalObject, source: EvalObject): void {
  for (const [key, entry] of source.entries) {
    const existing = target.entries.get(key);
    if (!existing) {
      target.set(key, { value: cloneValue(entry.value), layer: "base", kind: "ordinary" });
    } else if (existing.layer === "base") {
      overlayLocal(existing, cloneValue(entry.value), entry.kind);
    }
  }
}

function overlayLocal(existing: EvalEntry, value: SconValue | EvalObject, kind: Kind): void {
  if (existing.value instanceof EvalObject && value instanceof EvalObject) {
    mergeOverride(existing.value, value);
    existing.layer = "local";
    existing.kind = kind;
  } else {
    existing.value = value;
    existing.layer = "local";
    existing.kind = kind;
  }
}

function mergeOverride(target: EvalObject, source: EvalObject): void {
  for (const [key, entry] of source.entries) {
    const existing = target.entries.get(key);
    if (existing?.value instanceof EvalObject && entry.value instanceof EvalObject) {
      mergeOverride(existing.value, entry.value);
    } else {
      target.set(key, { value: cloneValue(entry.value), layer: entry.layer, kind: entry.kind });
    }
  }
}

function samePath(a: string[], b: string[]): boolean {
  return a.length === b.length && a.every((item, index) => item === b[index]);
}

function invalidIncludePath(path: string): boolean {
  return hasPathControlChar(path) ||
    path.includes("://") ||
    path.startsWith("classpath:") ||
    path.includes("*") ||
    path.startsWith("~") ||
    path.startsWith("$") ||
    isAbsolute(path) ||
    /^[A-Za-z]:[\\/]/.test(path);
}

function hasPathControlChar(path: string): boolean {
  for (const ch of path) {
    if (ch.charCodeAt(0) < 0x20) return true;
  }
  return false;
}

function escapesIncludeRoot(path: string): boolean {
  return path.split(/[\\/]+/).some((segment) => segment === "..");
}

function withinRoot(path: string, root: string): boolean {
  const normalizedPath = normalize(path);
  const normalizedRoot = normalize(root);
  const relative = normalizedPath.slice(normalizedRoot.length);
  return normalizedPath === normalizedRoot || (!!relative && !relative.startsWith(".."));
}

function limit(value: number | undefined, fallback: number): number {
  return value ?? fallback;
}
