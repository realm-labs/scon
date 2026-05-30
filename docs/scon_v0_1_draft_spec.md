# SCON v0.1 Draft Specification

**Title:** SCON: Strict Configuration Object Notation  
**Status:** Draft v0.1  
**Resolution model:** single sequential model; no profiles  
**Audience:** parser implementers, runtime maintainers, tool authors  
**Last edited:** 2026-05-30

This document is self-contained. It does not depend on any previous draft, compatibility profile, or external version history.

SCON is a strict, HOCON-like configuration format designed for deterministic behavior, simple parsers, high parsing performance, and consistent cross-platform implementations.

The central design decision in this draft is:

```text
Substitution and spread are resolved immediately against the current resolved state.
Forward references are not allowed.
There is only one conforming implementation model.
```

This removes the need for dependency graphs, topological sorting, substitution cycle resolution, self-reference fallback, and “final effective value” ambiguity.

---

## 1. Design Goals

SCON aims to provide:

1. **Determinism**: the same files produce the same resolved tree and errors in every conforming implementation.
2. **Simple parsing**: a hand-written recursive descent parser should be sufficient.
3. **High performance**: lexing and parsing are linear; resolution is a single ordered evaluation pass plus merge work.
4. **Explicit composition**: include, object spread, and array spread are explicit.
5. **Static keys**: substitutions cannot create keys.
6. **Single semantics**: no compatibility modes, no profiles, no optional resolution behavior.

SCON intentionally does not provide:

- bare strings;
- implicit string/value concatenation;
- implicit object concatenation;
- implicit array concatenation;
- forward substitution references;
- self-reference fallback;
- environment-variable lookup in the core language;
- URL, classpath, or glob includes;
- alternate implementation profiles.

---

## 2. Data Model

SCON has exactly six data types:

```text
null
boolean
number
string
array
object
```

There are no native date, duration, size, regex, binary, enum, symbol, or tagged types. Such values should be represented as strings and interpreted by schema or application code.

A document represents one root object. Root braces may be omitted.

Valid:

```scon
name = "demo"
port = 8080
```

Equivalent to:

```scon
{
  name = "demo"
  port = 8080
}
```

Invalid as a document root:

```scon
[1, 2, 3]
```

---

## 3. Encoding, Whitespace, and Comments

A SCON source file must be UTF-8.

A conforming implementation must accept:

```text
LF
CRLF
```

`CRLF` must be normalized to `LF`. A standalone `CR` is invalid.

Outside strings, only these whitespace characters are allowed:

```text
space  U+0020
tab    U+0009
LF     U+000A
```

Other Unicode whitespace characters are invalid outside strings, including full-width spaces and non-breaking spaces.

The field separator `=` may have any number of spaces or tabs around it.

Valid:

```scon
name="demo"
name = "demo"
name     =     "demo"
```

Invalid:

```scon
name
= "demo"
```

Invalid:

```scon
name =
"demo"
```

SCON supports line comments:

```scon
# comment
// comment
```

Block comments are not supported.

---

## 4. Keys and Paths

An unquoted key segment must match:

```text
[A-Za-z_][A-Za-z0-9_-]*
```

Valid:

```scon
name = "demo"
server_port = 8080
log-level = "info"
```

A quoted key segment may be used for special characters:

```scon
"http.port" = 8080

server {
  "read.only" = true
}
```

Path keys are supported:

```scon
server.host = "127.0.0.1"
server.port = 8080
```

Equivalent to:

```scon
server {
  host = "127.0.0.1"
  port = 8080
}
```

Quoted path segments are allowed:

```scon
a."b.c".d = 1
```

This represents:

```json
{
  "a": {
    "b.c": {
      "d": 1
    }
  }
}
```

Substitution is not allowed in keys.

Invalid:

```scon
${name} = 1
server.${field} = 8080
```

---

## 5. Objects

Explicit object:

```scon
server = {
  host = "127.0.0.1"
  port = 8080
}
```

Object shorthand:

```scon
server {
  host = "127.0.0.1"
  port = 8080
}
```

The path and `{` must be on the same logical line.

Valid:

```scon
server     {
  port = 8080
}
```

Invalid:

```scon
server
{
  port = 8080
}
```

Object members may be separated by newlines or commas.

Valid:

```scon
server {
  host = "127.0.0.1"
  port = 8080
}
```

Valid:

```scon
server = { host = "127.0.0.1", port = 8080 }
```

Trailing commas are allowed:

```scon
server = {
  host = "127.0.0.1",
  port = 8080,
}
```

Consecutive commas are invalid.

Object spread expressions must appear before include directives and regular fields in the same object body.

Valid:

```scon
prod {
  ...${defaults}
  ...${platform_defaults}

  port = 9090
}
```

Invalid:

```scon
prod {
  port = 9090
  ...${defaults}
}
```

---

## 6. Arrays

Array elements must be separated by commas.

Valid:

```scon
ports = [80, 443, 8080]

paths = [
  "/bin",
  "/usr/bin",
  "/opt/bin",
]
```

Invalid:

```scon
ports = [80 443 8080]
```

Arrays do not support implicit concatenation. Use array spread for explicit expansion.

---

## 7. Strings and Numbers

Strings must be double-quoted. Bare strings are invalid.

Valid:

```scon
name = "demo"
path = "/usr/local/bin"
level = "info"
```

Invalid:

```scon
name = demo
path = /usr/local/bin
level = info
```

Strings use JSON-style escapes:

```text
\"  quote
\\  backslash
\/  slash
\b  backspace
\f  form feed
\n  line feed
\r  carriage return
\t  tab
\uXXXX  Unicode escape
```

SCON additionally supports `\$` for literal interpolation syntax:

```scon
text = "literal \${name}"
```

Resolved string:

```text
literal ${name}
```

Multiline string syntax is not supported. Use `\n` explicitly.

Number syntax is JSON-like:

```scon
int = 123
neg = -10
float = 3.14
exp = 1.5e6
```

Invalid:

```scon
nan = NaN
inf = Infinity
hex = 0xFF
leading = 0123
```

The core specification defines number syntax, not a required in-memory numeric type.

---

## 8. Substitution

A substitution has the form:

```scon
copy = ${some.path}
```

A full-value substitution preserves the referenced value type.

Example:

```scon
base = {
  host = "127.0.0.1"
  port = 8080
}

copy = ${base}
```

`copy` resolves to an object.

Substitution paths are absolute paths from the root object. Relative substitution paths are not supported.

Invalid:

```scon
url = "http://${.host}"
url = "http://${../host}"
```

### 8.1 Sequential Resolution

SCON has one substitution rule:

```text
A substitution is resolved immediately when the containing value is evaluated.
It may only reference a completed value already present in the current resolved state.
```

A substitution must not reference:

- a value defined later in source order;
- a value that appears later in an included file that has not been evaluated yet;
- the value currently being defined;
- an object body currently being evaluated as a whole;
- a previous version of a value;
- an implementation-defined temporary value.

Valid:

```scon
host = "127.0.0.1"
port = 8080
url = "http://${host}:${port}"
```

Invalid:

```scon
url = "http://${host}:${port}"
host = "127.0.0.1"
port = 8080
```

The reference to `host` is invalid because `host` is not defined before use.

### 8.2 References Inside the Current Object

While an object body is being evaluated, previously completed members of that object are visible at their absolute paths.

Valid:

```scon
server {
  host = "127.0.0.1"
  port = 8080
  url = "http://${server.host}:${server.port}"
}
```

Invalid:

```scon
server {
  url = "http://${server.host}:${server.port}"
  host = "127.0.0.1"
  port = 8080
}
```

The derived field appears before its dependencies.

A completed child value may be referenced, but the object currently being evaluated is not itself complete.

Valid:

```scon
server {
  tls {
    enabled = true
  }

  tls_copy = ${server.tls}
}
```

Invalid:

```scon
server {
  host = "127.0.0.1"
  copy = ${server}
}
```

`server` is still in progress while its own body is being evaluated.

### 8.3 No Forward References

Forward references are not allowed.

Invalid:

```scon
url = "http://${host}:${port}"
host = "127.0.0.1"
port = 8080
```

Correct:

```scon
host = "127.0.0.1"
port = 8080
url = "http://${host}:${port}"
```

This rule removes the need for dependency graphs, topological sorting, and substitution cycle resolution.

### 8.4 No Previous-Value Lookup

SCON does not support HOCON-style lookup of a previously defined value.

Invalid:

```scon
path = ["/bin"]
path = [${path}, "/usr/bin"]
```

This is a duplicate local leaf definition, not an append operation.

Correct:

```scon
base_path = ["/bin"]

path = [
  ...${base_path},
  "/usr/bin",
]
```

### 8.5 No Self-Reference Fallback

Self-reference is invalid.

Invalid:

```scon
value = ${value}
```

A conforming implementation should report this as a reference to a value that is not completed before use.

### 8.6 No Optional or Environment Substitution

SCON does not support optional substitutions:

```scon
password = ${?DB_PASSWORD}
```

SCON does not support inline defaults:

```scon
port = ${APP_PORT:-8080}
```

SCON core does not read environment variables. A host may inject an `env` object before evaluating the entry file, but that is outside this core specification.

### 8.7 Lookup-Time Snapshot

A substitution or spread observes the value at lookup time. Later structural additions to the referenced object do not retroactively update already resolved values.

```scon
base {
  host = "0.0.0.0"
}

prod {
  ...${base}
}

base {
  port = 8080
}
```

Resolved result:

```json
{
  "base": {
    "host": "0.0.0.0",
    "port": 8080
  },
  "prod": {
    "host": "0.0.0.0"
  }
}
```

`prod` does not receive `base.port` because `base.port` was added after the spread was evaluated.

Derived fields and reusable defaults should be fully defined before use.

---

## 9. String Interpolation

A substitution may appear inside a string:

```scon
host = "127.0.0.1"
port = 8080
url = "http://${host}:${port}"
```

Interpolation is resolved immediately when the string value is evaluated.

Allowed interpolation value types:

```text
string
number
boolean
```

Disallowed interpolation value types:

```text
object
array
null
```

Invalid:

```scon
obj = { a = 1 }
x = "value = ${obj}"
```

Invalid:

```scon
x = null
y = "value = ${x}"
```

---

## 10. Object Spread and Deep Overlay Merge

Object spread is the explicit object inheritance mechanism.

```scon
prod {
  ...${defaults}

  port = 9090
}
```

The target of object spread must already be a completed object at the point where the spread appears. Otherwise, evaluation fails with `MissingReference` or `TypeMismatch`.

Object spread has the form:

```scon
...${path}
```

It is only valid inside object bodies.

Object spreads must appear before all include directives and regular fields in the same object body.

### 10.1 Base Layer and Local Layer

An object body is evaluated in this order:

```text
1. Start with an empty object at the target path.
2. Apply object spreads in source order as the base layer using deep overlay merge.
3. Evaluate includes and regular fields in source order as the local layer.
4. Local values overlay base values.
5. Local-to-local leaf conflicts are errors.
```

This means local fields can override values inherited from spreads, but regular local fields cannot silently override each other.

### 10.2 Deep Overlay Merge

Overlay merge is used for object spread and for local values overriding spread values.

Algorithm:

```text
overlay_merge(left, right):
  for each key in right, in source order:
    if key does not exist in left:
      left[key] = right[key]
    else if left[key] is object and right[key] is object:
      left[key] = overlay_merge(left[key], right[key])
    else:
      left[key] = right[key]
  return left
```

Arrays are not deep-merged. A later array replaces an earlier value as a whole.

### 10.3 Multiple Spreads

Multiple object spreads are applied in source order. Later spreads overlay earlier spreads.

```scon
a {
  server {
    host = "0.0.0.0"
    port = 8080
  }
}

b {
  server {
    port = 9090
  }
}

c {
  ...${a}
  ...${b}
}
```

Resolved `c`:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 9090
  }
}
```

### 10.4 Spread Values Are Already Resolved

Because spread sources must be completed before use, substitutions inside a spread source are already resolved before the source object is spread.

```scon
base {
  host = "base.example"
  url = "http://${base.host}"
}

prod {
  ...${base}
  host = "prod.example"
}
```

Resolved `prod`:

```json
{
  "host": "prod.example",
  "url": "http://base.example"
}
```

`prod.url` is inherited as an already resolved value. It is not dynamically rebound to `prod.host`.

To derive a value from the target object, define the derived value after its dependencies in the target object:

```scon
base {
  host = "base.example"
}

prod {
  ...${base}

  host = "prod.example"
  url = "http://${prod.host}"
}
```

### 10.5 Full-Value Substitution Is Not Object Spread

A full-value substitution that resolves to an object does not participate in structural merge. Use object spread for object inheritance.

Invalid:

```scon
base {
  server {
    host = "0.0.0.0"
    port = 8080
  }
}

server = ${base.server}

server {
  port = 9090
}
```

Correct:

```scon
server {
  ...${base.server}
  port = 9090
}
```

---

## 11. Array Spread

Array spread is the explicit array expansion mechanism.

```scon
base_paths = [
  "/bin",
  "/usr/bin",
]

paths = [
  ...${base_paths},
  "/opt/app/bin",
]
```

The target of array spread must already be a completed array at the point where the spread appears.

A normal substitution inside an array does not flatten arrays.

```scon
base = [1, 2]
x = [${base}, 3]
```

Resolved result:

```json
{
  "x": [[1, 2], 3]
}
```

To flatten, use array spread:

```scon
x = [...${base}, 3]
```

Resolved result:

```json
{
  "x": [1, 2, 3]
}
```

Array elements are evaluated left-to-right. The array value itself is not completed until all elements have been evaluated.

---

## 12. Include

Include syntax:

```scon
include "./file.scon"
```

The path must be a double-quoted string literal. It must not contain interpolation.

Invalid:

```scon
include ${file}
include "./${env}.scon"
include database.scon
```

SCON core supports only local files. It does not support URL, classpath, glob, home-directory, or environment-variable expansion.

Invalid:

```scon
include "https://example.com/config.scon"
include "classpath:app.conf"
include "*.scon"
include "~/config.scon"
include "$HOME/config.scon"
```

Relative include paths are resolved relative to the file containing the include directive.

The core specification rejects absolute include paths.

Invalid:

```scon
include "/etc/app/config.scon"
include "C:/app/config.scon"
```

A loader should define an `include_root`. The canonical path of every included file must remain inside the include root. An include that escapes the include root must be rejected.

Include directives are only valid inside object bodies.

Valid:

```scon
include "./database.scon"

app {
  include "./app-base.scon"
  name = "demo"
}
```

Invalid:

```scon
x = include "./base.scon"
```

Invalid:

```scon
items = [
  include "./items.scon"
]
```

An included file must represent an object at its root.

Include is evaluated as an object fragment inserted into the current object at the include position. It is not source-text substitution.

Example:

```scon
# app.scon
server {
  include "./server-prod.scon"
}
```

```scon
# server-prod.scon
host = "0.0.0.0"
port = 8080
```

Resolved result:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  }
}
```

Includes are evaluated in source order.

Valid:

```scon
defaults {
  host = "0.0.0.0"
}

include "./server.scon"
```

```scon
# server.scon
server {
  host = ${defaults.host}
}
```

Invalid:

```scon
include "./server.scon"

defaults {
  host = "0.0.0.0"
}
```

If `server.scon` references `${defaults.host}`, the reference is invalid because `defaults.host` is not completed before the include.

Include cycles must be rejected:

```text
IncludeCycle: a.scon -> b.scon -> a.scon
```

The same file may be included more than once. Each include directive is evaluated at its own source position. Parsed ASTs may be cached; resolved include results generally should not be globally cached because evaluation depends on source order and current state.

---

## 13. Sequential Composition and Resolution

A conforming implementation should model processing as:

```text
1. Decode UTF-8 source.
2. Normalize CRLF to LF.
3. Lex and parse source files into ASTs.
4. Evaluate the entry document root object from top to bottom.
5. When an include directive is encountered, load and evaluate the included root object at that point.
6. When an object or array spread is encountered, immediately look up the completed target.
7. When a substitution or interpolation is encountered, immediately look up the completed target.
8. Insert or merge the evaluated value into the current resolved state.
9. Produce the resolved root object.
```

The parser does not resolve substitutions. The evaluator resolves them immediately when evaluating values.

The current resolved state is the value tree built so far. A path lookup succeeds only when the exact path resolves to a completed value. Missing values, future values, and in-progress objects are not visible as completed values.

### 13.1 In-Progress Objects

When evaluating an object body at path `P`, `P` is in progress until the closing brace of that object body is reached.

Previously completed child paths of `P` are visible:

```scon
server {
  host = "127.0.0.1"
  url = "http://${server.host}"
}
```

But the object `server` itself is not completed until after its body ends:

```scon
server {
  host = "127.0.0.1"
  copy = ${server}
}
```

This is invalid.

### 13.2 Structural Merge

Structural merge combines regular object fragments from:

- repeated syntactic object fields;
- path key expansion;
- include directives;
- regular local fields within the same object.

Structural merge is a deep merge that only allows object/object merging.

Algorithm:

```text
structural_merge(left, right, path):
  for each key in right, in source order:
    if key does not exist in left:
      left[key] = right[key]
    else if left[key] is a syntactic local object
         and right[key] is a syntactic local object:
      structural_merge(left[key], right[key], path + key)
    else:
      error DuplicateKey or PathConflict at path + key
```

Eligibility for structural merge is based on syntax, not on runtime substitution results.

### 13.3 Overlay Merge

Overlay merge is used for:

- applying object spreads in source order;
- allowing local values to override spread-provided base values.

Overlay merge is not used to resolve conflicts between two regular local leaf fields.

### 13.4 Source Order Matters

Because SCON uses sequential evaluation, source order is semantically meaningful.

```scon
base {
  host = "0.0.0.0"
  port = 8080
}

server {
  ...${base}

  url = "http://${server.host}:${server.port}"
  port = 9090
}
```

Resolved `server.url` is:

```text
http://0.0.0.0:8080
```

To make `url` observe the local override, write the dependency first:

```scon
server {
  ...${base}

  port = 9090
  url = "http://${server.host}:${server.port}"
}
```

Resolved `server.url` is:

```text
http://0.0.0.0:9090
```

Derived fields should be written after the fields they depend on.

---

## 14. Duplicate Keys and Path Conflicts

Local leaf duplicates are errors.

Invalid:

```scon
server {
  port = 8080
  port = 9090
}
```

Repeated syntactic object fields may merge:

```scon
server {
  host = "0.0.0.0"
}

server {
  port = 8080
}
```

Resolved result:

```json
{
  "server": {
    "host": "0.0.0.0",
    "port": 8080
  }
}
```

Repeated leaf values inside merged objects are errors:

```scon
server {
  port = 8080
}

server {
  port = 9090
}
```

Scalar/object conflicts are errors:

```scon
server = "localhost"
server.port = 8080
```

---

## 15. Error Handling

A conforming implementation must report errors. It must not silently recover in a way that changes meaning.

An error should include:

```text
error code
message
file
line
column
path, if applicable
include stack, if applicable
hint, optional
```

Recommended error codes:

```text
InvalidCharacter
InvalidWhitespace
InvalidEscape
UnexpectedToken
UnterminatedString
InvalidNumber
InvalidRootType
DuplicateKey
PathConflict
MissingReference
TypeMismatch
InvalidSpread
InvalidIncludePath
IncludeNotFound
IncludeNotFile
IncludePathDenied
IncludeCycle
IncludeParseError
IncludeRootTypeError
ResourceLimitExceeded
```

`MissingReference` covers references to values that are missing, defined later, or not yet fully completed at the point of use. Implementations may provide a more specific diagnostic message while preserving the same observable failure.

Example:

```text
MissingReference at config/app.scon:8:18
path: server.host
message: path "server.host" is not defined before use
```

Example:

```text
DuplicateKey at config/app.scon:12:3
path: server.port
message: key "server.port" is already defined in the local layer
```

---

## 16. Normative EBNF

Whitespace and comments are simplified for readability.

```ebnf
document        = ws, object_body?, ws, EOF ;

object          = "{", ws, object_body?, ws, "}" ;

object_body     = object_spread_section?, local_member_section? ;

object_spread_section
                = object_spread, (member_sep, object_spread)*, member_sep? ;

local_member_section
                = local_member, (member_sep, local_member)*, member_sep? ;

local_member    = include_directive
                | field ;

include_directive
                = "include", hws1, string_no_interpolation ;

field           = path, hws, "=", hws, value
                | path, hws, object ;

value           = object
                | array
                | string
                | number
                | boolean
                | null
                | substitution ;

array           = "[", ws, array_items?, ws, "]" ;

array_items     = array_item, (ws, ",", ws, array_item)*, (ws, ",")? ;

array_item      = array_spread
                | value ;

object_spread   = "...", hws?, substitution ;

array_spread    = "...", hws?, substitution ;

substitution    = "${", path, "}" ;

path            = path_segment, (".", path_segment)* ;

path_segment    = identifier
                | quoted_key ;

identifier      = ALPHA_OR_UNDERSCORE, { ALPHA_OR_DIGIT_OR_UNDERSCORE_OR_HYPHEN } ;

quoted_key      = string_no_interpolation ;

boolean         = "true" | "false" ;

null            = "null" ;

number          = [ "-" ], int, [ frac ], [ exp ] ;

hws             = { " " | "\t" } ;
hws1            = ( " " | "\t" ), hws ;
ws              = { hws | newline | comment } ;
newline         = "\n" ;
```

An implementation must reject an `object_spread` that appears after any `local_member` in the same object body.

There is no `bare_string` production.

The only field separator is `=`. The `:` separator is not part of v0.1.

---

## 17. Recommended Implementation Architecture

A conforming implementation can be organized into four components:

```text
Lexer
Parser
Include Loader
Sequential Evaluator
```

### 17.1 Lexer

Recommended token set:

```text
IDENT
STRING
NUMBER
TRUE
FALSE
NULL
INCLUDE
LBRACE       {
RBRACE       }
LBRACKET     [
RBRACKET     ]
EQUAL        =
COMMA        ,
DOT          .
ELLIPSIS     ...
SUBST_START  ${
NEWLINE
END_OF_FILE
```

Comments are skipped, but their terminating newline remains visible as `NEWLINE`.

### 17.2 Parser

The parser produces an AST and does not resolve substitutions, load includes, or access the filesystem.

Recommended AST shape:

```text
Document
  body: ObjectBody

ObjectBody
  spreads: ObjectSpread[]
  local_members: LocalMember[]

LocalMember
  Include(path_string)
  Field(path, value, syntax_kind)

Value
  Object(ObjectBody)
  Array(ArrayItem[])
  String(StringPart[])
  Number(raw_text)
  Boolean
  Null
  Substitution(path)

ArrayItem
  Value(value)
  ArraySpread(substitution)
```

After parsing a path inside an object body, the next significant token decides the construct:

```text
=     field assignment
{     object shorthand
other error
```

No backtracking is required.

### 17.3 Include Loader

The include loader should:

- resolve relative paths against the including file directory;
- canonicalize paths;
- enforce include root containment;
- reject absolute paths in core mode;
- reject include cycles using an include stack;
- cache parsed ASTs by canonical path;
- preserve source locations for diagnostics.

Parse caching is recommended. Resolved-value caching for included files is generally unsafe because include evaluation depends on source order, current target object, and current resolved state.

### 17.4 Sequential Evaluator

Recommended state representation:

```text
ResolvedState
  root: ObjectValue
  path_index: trie or hash map from absolute path to Entry

Entry
  value: Value
  status: Complete | InProgressObject
  layer: Base | Local
  merge_kind: StructuralObject | OrdinaryValue
  source_location
```

Object evaluation:

```text
evaluate_object_body(body, path):
  mark path as InProgressObject

  for spread in body.spreads:
    base = lookup_completed(spread.path)
    require_object(base)
    overlay_into(path, base, layer = Base)

  for member in body.local_members:
    if member is Include:
      ast = load_include(member.path)
      evaluate_object_body_as_fragment(ast.root_body, path)
    else if member is Field:
      evaluate_field(member, path)

  mark path as Complete
```

Field evaluation:

```text
evaluate_field(field, current_object_path):
  target_path = current_object_path + field.path

  if field.value is syntactic object:
    ensure_structural_object_slot(target_path)
    evaluate_object_body(field.value.body, target_path)
  else:
    value = evaluate_value(field.value)
    insert_local_value(target_path, value)
```

Substitution evaluation:

```text
evaluate_substitution(path):
  entry = lookup(path)

  if entry does not exist:
    error MissingReference

  if entry.status is not Complete:
    error MissingReference

  return entry.value
```

No dependency graph is required.

Merge conflict handling should distinguish layers:

```text
Base + Base     -> overlay merge
Base + Local    -> local overlays base
Local + Local   -> structural merge only; duplicate leaf is error
```

---

## 18. High-Performance Implementation Guidance

### 18.1 Keep Lexing and Parsing Linear

A fast implementation should:

- scan UTF-8 once;
- normalize newlines during lexing;
- reject invalid whitespace immediately;
- intern identifier segments when useful;
- parse numbers as raw slices first;
- parse strings into raw slices when no escapes are present;
- decode escaped strings only when necessary.

### 18.2 Use a Path Trie or Interned Path Table

Substitution and spread lookup should be fast. Recommended approaches:

- trie keyed by interned path segments;
- hash map keyed by compact path IDs;
- two-level map for common top-level keys.

Avoid repeatedly splitting path strings during evaluation.

### 18.3 Copy-on-Write Overlay Merge

Naive deep overlay can copy large objects repeatedly.

For high-performance object spread, use immutable values or copy-on-write objects:

```text
1. Share unchanged subtrees from the spread source.
2. Clone only object nodes on paths that are overridden.
3. Store scalar and array values by reference when immutable.
```

Example:

```scon
prod {
  ...${huge_defaults}

  server {
    port = 9090
  }
}
```

Only the root object and the `server` object need to be cloned. Unchanged subtrees can be shared.

### 18.4 Complexity

For total input bytes `N`, object entries `M`, substitutions/spreads `R`, and include bytes `I`:

```text
lex + parse:             O(N + I)
include path handling:   O(number of include directives)
sequential evaluation:   O(M + R + merge work)
path lookup:             O(path length) with trie, or near O(1) with path IDs
```

The main benefit of the v0.1 model is that evaluation does not require:

- dependency graph construction;
- topological sorting;
- global substitution cycle detection;
- lazy resolution of future values;
- final effective-value recomputation.

A substitution is a lookup against the current resolved state.

---

## 19. Security and Resource Limits

Implementations should provide configurable limits:

```text
maximum file size
maximum total included bytes
maximum include depth
maximum number of include files
maximum object nesting depth
maximum array length
maximum string length
maximum path length
maximum number of object members
```

The loader must reject include cycles.
The loader should reject include paths that escape the include root.
The core language must not fetch URLs or execute commands.

---

## 20. Canonical Formatting Recommendations

A formatter should use:

```text
indentation: 2 spaces
field separator: =
strings: always double-quoted
line endings: LF
file ending: one final LF
object spread: first in object body
include directives: after spreads, before ordinary fields when possible
arrays: multiline for long arrays, one item per line
trailing commas: allowed, formatter may choose consistently
```

The formatter should preserve field order. It should not sort keys by default because source order is semantically meaningful.

---

## 21. Summary of Core Rules

```text
1. A SCON document is a root object.
2. Strings must be double-quoted; bare strings are invalid.
3. The only field separator is =.
4. Horizontal whitespace around = is unrestricted, but newlines are not allowed there.
5. Arrays require comma separators.
6. Object members may be separated by newlines or commas.
7. Include only supports local files with literal string paths.
8. Include is evaluated in source order as an object fragment, not as source text.
9. Object spread must appear before local members in an object body.
10. Object spread uses deep overlay merge.
11. Regular object fragments use structural deep merge.
12. Local leaf duplicates are errors.
13. Substitution is resolved immediately against the current resolved state.
14. Forward references are invalid.
15. Self-reference fallback does not exist.
16. Object and array spreads require already completed targets.
17. Source order is semantically meaningful.
18. There is only one conforming resolution model.
```
