import type { Span } from "./error.js";

export type Document = {
  root: AstObject;
  file?: string;
};

export type AstObject = {
  members: AstMember[];
  span: Span;
};

export type AstMember = AstField | AstInclude | AstObjectSpread;

export type AstField = {
  type: "field";
  path: AstPath;
  value: AstValue;
  span: Span;
};

export type AstInclude = {
  type: "include";
  path: AstString;
  span: Span;
};

export type AstObjectSpread = {
  type: "objectSpread";
  sub: AstSubstitution;
  span: Span;
};

export type AstPath = {
  segments: AstPathSegment[];
  span: Span;
};

export type AstPathSegment = {
  value: string;
  quoted: boolean;
  span: Span;
};

export type AstValue =
  | AstNull
  | AstBool
  | AstNumberNode
  | AstString
  | AstArray
  | AstObjectValue
  | AstSubstitution;

export type AstNull = { type: "null"; span: Span };
export type AstBool = { type: "bool"; value: boolean; span: Span };
export type AstNumberNode = { type: "number"; raw: string; span: Span };

export type AstString = {
  type: "string";
  value: string;
  raw: string;
  parts: StringPart[];
  span: Span;
};

export type AstArray = {
  type: "array";
  items: AstArrayItem[];
  span: Span;
};

export type AstObjectValue = {
  type: "object";
  object: AstObject;
  span: Span;
};

export type AstSubstitution = {
  type: "substitution";
  path: AstPath;
  span: Span;
};

export type AstArrayItem =
  | { type: "value"; value: AstValue; span: Span }
  | { type: "spread"; sub: AstSubstitution; span: Span };

export type StringPart =
  | { type: "literal"; value: string }
  | { type: "interpolation"; path: AstPath; span: Span };
