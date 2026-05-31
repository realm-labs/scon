export type ErrorCode =
  | "InvalidCharacter"
  | "InvalidWhitespace"
  | "InvalidEscape"
  | "UnexpectedToken"
  | "UnterminatedString"
  | "InvalidNumber"
  | "InvalidRootType"
  | "DuplicateKey"
  | "PathConflict"
  | "MissingReference"
  | "TypeMismatch"
  | "InvalidSpread"
  | "InvalidIncludePath"
  | "IncludeNotFound"
  | "IncludeNotFile"
  | "IncludePathDenied"
  | "IncludeCycle"
  | "IncludeParseError"
  | "IncludeRootTypeError"
  | "ResourceLimitExceeded"
  | "Serde";

export type Span = {
  start: number;
  end: number;
};

export class SconError extends Error {
  constructor(
    readonly code: ErrorCode,
    message: string,
    readonly span?: Span,
  ) {
    super(message);
  }
}

export function fail(code: ErrorCode, message: string, start: number, end: number): never {
  throw new SconError(code, message, { start, end });
}
