import type { AstMember, AstObject, AstPath, AstValue, Document } from "./ast.js";
import type { ErrorCode, Span } from "./error.js";
import { SconError } from "./error.js";
import { lex, type Token } from "./lexer.js";
import { parseDocument } from "./parser.js";
import { parseString } from "./resolver.js";
import type { SconValue } from "./value.js";

export type DiagnosticSeverity = "error" | "warning" | "information" | "hint";
export type SourcePosition = { line: number; column: number };
export type SourceRange = { start: SourcePosition; end: SourcePosition; span: Span };
export type Comment = { text: string; span: Span; range: SourceRange };
export type Diagnostic = {
  code: ErrorCode;
  message: string;
  severity: DiagnosticSeverity;
  file?: string;
  range?: SourceRange;
};
export type Symbol = { path: string[]; file?: string; range: SourceRange };
export type Definition = { path: string[]; file?: string; range: SourceRange };
export type ReferenceKind = "substitution" | "interpolation" | "objectSpread" | "arraySpread";
export type Reference = { path: string[]; kind: ReferenceKind; file?: string; range: SourceRange; target?: Definition };
export type IncludeReference = { path: string; file?: string; range: SourceRange; resolvedPath?: string };
export type ParsedSource = {
  file?: string;
  tokens: Array<Token & { range: SourceRange }>;
  comments: Comment[];
  symbols: Symbol[];
};
export type Analysis = {
  file?: string;
  parsed?: ParsedSource;
  diagnostics: Diagnostic[];
  comments: Comment[];
  symbols: Symbol[];
  definitions: Definition[];
  references: Reference[];
  includes: IncludeReference[];
  value?: SconValue;
};

export function parseSource(source: string, file?: string): ParsedSource {
  const document = parseDocument(source, file);
  const lineIndex = new LineIndex(source);
  const tokens = lex(source).map((token) => ({ ...token, range: lineIndex.range(token.span) }));
  return {
    file,
    tokens,
    comments: comments(tokens),
    symbols: collectSymbols(document.root, lineIndex, file, []),
  };
}

export function analyzeSource(source: string, file?: string): Analysis {
  const lineIndex = new LineIndex(source);
  let tokens: Array<Token & { range: SourceRange }> = [];
  try {
    tokens = lex(source).map((token) => ({ ...token, range: lineIndex.range(token.span) }));
    const document = parseDocument(source, file);
    const parsed = {
      file,
      tokens,
      comments: comments(tokens),
      symbols: collectSymbols(document.root, lineIndex, file, []),
    };
    const definitions = collectDefinitions(document.root, lineIndex, file, []);
    const references = collectReferences(document, lineIndex, file);
    resolveTargets(references, definitions);
    let value: SconValue | undefined;
    const diagnostics: Diagnostic[] = [];
    try {
      value = parseString(source);
    } catch (err) {
      diagnostics.push(diagnosticFromError(err, lineIndex, file));
    }
    return {
      file,
      parsed,
      diagnostics,
      comments: parsed.comments,
      symbols: parsed.symbols,
      definitions,
      references,
      includes: collectIncludes(document.root, lineIndex, file),
      value,
    };
  } catch (err) {
    return {
      file,
      diagnostics: [diagnosticFromError(err, lineIndex, file)],
      comments: comments(tokens),
      symbols: [],
      definitions: [],
      references: [],
      includes: [],
    };
  }
}

function collectSymbols(object: AstObject, lineIndex: LineIndex, file: string | undefined, prefix: string[]): Symbol[] {
  return object.members.flatMap((member) => {
    if (member.type !== "field") return [];
    const path = [...prefix, ...pathNames(member.path)];
    const nested = member.value.type === "object" ? collectSymbols(member.value.object, lineIndex, file, path) : [];
    return [{ path, file, range: lineIndex.range(member.path.span) }, ...nested];
  });
}

function collectDefinitions(object: AstObject, lineIndex: LineIndex, file: string | undefined, prefix: string[]): Definition[] {
  return object.members.flatMap((member) => {
    if (member.type !== "field") return [];
    const path = [...prefix, ...pathNames(member.path)];
    const nested = member.value.type === "object" ? collectDefinitions(member.value.object, lineIndex, file, path) : [];
    return [{ path, file, range: lineIndex.range(member.path.span) }, ...nested];
  });
}

function collectReferences(document: Document, lineIndex: LineIndex, file?: string): Reference[] {
  return collectObjectReferences(document.root, lineIndex, file);
}

function collectObjectReferences(object: AstObject, lineIndex: LineIndex, file?: string): Reference[] {
  return object.members.flatMap((member) => {
    switch (member.type) {
      case "objectSpread":
        return [reference(member.sub.path, "objectSpread", lineIndex, file)];
      case "field":
        return collectValueReferences(member.value, lineIndex, file);
      case "include":
        return [];
    }
  });
}

function collectValueReferences(value: AstValue, lineIndex: LineIndex, file?: string): Reference[] {
  switch (value.type) {
    case "substitution":
      return [reference(value.path, "substitution", lineIndex, file)];
    case "string":
      return value.parts.flatMap((part) => part.type === "interpolation" ? [reference(part.path, "interpolation", lineIndex, file)] : []);
    case "array":
      return value.items.flatMap((item) => item.type === "spread"
        ? [reference(item.sub.path, "arraySpread", lineIndex, file)]
        : collectValueReferences(item.value, lineIndex, file));
    case "object":
      return collectObjectReferences(value.object, lineIndex, file);
    default:
      return [];
  }
}

function collectIncludes(object: AstObject, lineIndex: LineIndex, file?: string): IncludeReference[] {
  return object.members.flatMap((member: AstMember) => {
    if (member.type === "include") return [{ path: member.path.value, file, range: lineIndex.range(member.span) }];
    if (member.type === "field" && member.value.type === "object") return collectIncludes(member.value.object, lineIndex, file);
    return [];
  });
}

function reference(path: AstPath, kind: ReferenceKind, lineIndex: LineIndex, file?: string): Reference {
  return { path: pathNames(path), kind, file, range: lineIndex.range(path.span) };
}

function resolveTargets(references: Reference[], definitions: Definition[]): void {
  const byPath = new Map(definitions.map((definition) => [definition.path.join("\0"), definition]));
  for (const reference of references) {
    reference.target = byPath.get(reference.path.join("\0"));
  }
}

function comments(tokens: Array<Token & { range: SourceRange }>): Comment[] {
  return tokens.filter((token) => token.kind === "comment").map((token) => ({
    text: token.text,
    span: token.span,
    range: token.range,
  }));
}

function diagnosticFromError(err: unknown, lineIndex: LineIndex, file?: string): Diagnostic {
  if (err instanceof SconError) {
    return {
      code: err.code,
      message: err.message,
      severity: "error",
      file,
      range: err.span ? lineIndex.range(err.span) : undefined,
    };
  }
  return { code: "Serde", message: err instanceof Error ? err.message : String(err), severity: "error", file };
}

function pathNames(path: AstPath): string[] {
  return path.segments.map((segment) => segment.value);
}

class LineIndex {
  private readonly lines = [0];

  constructor(private readonly source: string) {
    for (let index = 0; index < source.length; index++) {
      if (source[index] === "\n") this.lines.push(index + 1);
    }
  }

  range(span: Span): SourceRange {
    return { start: this.position(span.start), end: this.position(span.end), span };
  }

  private position(offset: number): SourcePosition {
    let line = 0;
    while (line + 1 < this.lines.length && this.lines[line + 1]! <= offset) line++;
    return { line, column: offset - this.lines[line]! };
  }
}
