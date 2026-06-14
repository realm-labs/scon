package scon

type DiagnosticSeverity string

const (
	DiagnosticError       DiagnosticSeverity = "error"
	DiagnosticWarning     DiagnosticSeverity = "warning"
	DiagnosticInformation DiagnosticSeverity = "information"
	DiagnosticHint        DiagnosticSeverity = "hint"
)

type SourcePosition struct {
	Line   int
	Column int
}

type SourceRange struct {
	Start SourcePosition
	End   SourcePosition
	Span  Span
}

type Comment struct {
	Text  string
	Span  Span
	Range SourceRange
}

type Diagnostic struct {
	Code     ErrorCode
	Message  string
	Severity DiagnosticSeverity
	File     string
	Range    *SourceRange
}

type Symbol struct {
	Path  []string
	File  string
	Range SourceRange
}

type Definition struct {
	Path  []string
	File  string
	Range SourceRange
}

type ReferenceKind string

const (
	ReferenceSubstitution  ReferenceKind = "substitution"
	ReferenceInterpolation ReferenceKind = "interpolation"
	ReferenceObjectSpread  ReferenceKind = "objectSpread"
	ReferenceArraySpread   ReferenceKind = "arraySpread"
)

type Reference struct {
	Path   []string
	Kind   ReferenceKind
	File   string
	Range  SourceRange
	Target *Definition
}

type IncludeReference struct {
	Path         string
	File         string
	Range        SourceRange
	ResolvedPath string
}

type ParsedSource struct {
	File     string
	Tokens   []TokenInfo
	Comments []Comment
	Symbols  []Symbol
}

type TokenInfo struct {
	Kind  string
	Text  string
	Span  Span
	Range SourceRange
}

type Analysis struct {
	File        string
	Parsed      *ParsedSource
	Diagnostics []Diagnostic
	Comments    []Comment
	Symbols     []Symbol
	Definitions []Definition
	References  []Reference
	Includes    []IncludeReference
	Value       Value
}

type lineIndex struct {
	source string
	lines  []int
}

func ParseSource(source string) (*ParsedSource, error) {
	return ParseSourceFile(source, "")
}

func ParseSourceFile(source, file string) (*ParsedSource, error) {
	doc, err := parseDocument(source, file)
	if err != nil {
		return nil, err
	}
	tokens, err := lex(source)
	if err != nil {
		return nil, err
	}
	lines := newLineIndex(source)
	return &ParsedSource{
		File:     file,
		Tokens:   tokenInfos(tokens, lines),
		Comments: comments(tokens, lines),
		Symbols:  collectSymbols(doc.root, file, lines, nil),
	}, nil
}

func AnalyzeSource(source string) Analysis {
	return AnalyzeSourceFile(source, "")
}

func AnalyzeSourceFile(source, file string) Analysis {
	lines := newLineIndex(source)
	tokens, _ := lex(source)
	doc, err := parseDocument(source, file)
	if err != nil {
		return Analysis{
			File:        file,
			Diagnostics: []Diagnostic{diagnosticFromError(err, lines, file)},
			Comments:    comments(tokens, lines),
		}
	}
	value, resolveErr := ParseString(source)
	parsed := &ParsedSource{
		File:     file,
		Tokens:   tokenInfos(tokens, lines),
		Comments: comments(tokens, lines),
		Symbols:  collectSymbols(doc.root, file, lines, nil),
	}
	definitions := collectDefinitions(doc.root, file, lines, nil)
	references := collectReferences(doc.root, file, lines)
	resolveTargets(references, definitions)
	analysis := Analysis{
		File:        file,
		Parsed:      parsed,
		Comments:    parsed.Comments,
		Symbols:     parsed.Symbols,
		Definitions: definitions,
		References:  references,
		Includes:    collectIncludes(doc.root, file, lines),
		Value:       value,
	}
	if resolveErr != nil {
		analysis.Diagnostics = []Diagnostic{diagnosticFromError(resolveErr, lines, file)}
	}
	return analysis
}

func collectSymbols(object *astObject, file string, lines lineIndex, prefix []string) []Symbol {
	var symbols []Symbol
	for _, member := range object.members {
		field, ok := member.(astField)
		if !ok {
			continue
		}
		path := appendPath(prefix, pathNames(field.path)...)
		symbols = append(symbols, Symbol{Path: path, File: file, Range: lines.rangeOf(field.path.span)})
		if nested, ok := field.value.(astObjectValue); ok {
			symbols = append(symbols, collectSymbols(nested.object, file, lines, path)...)
		}
	}
	return symbols
}

func collectDefinitions(object *astObject, file string, lines lineIndex, prefix []string) []Definition {
	var definitions []Definition
	for _, member := range object.members {
		field, ok := member.(astField)
		if !ok {
			continue
		}
		path := appendPath(prefix, pathNames(field.path)...)
		definitions = append(definitions, Definition{Path: path, File: file, Range: lines.rangeOf(field.path.span)})
		if nested, ok := field.value.(astObjectValue); ok {
			definitions = append(definitions, collectDefinitions(nested.object, file, lines, path)...)
		}
	}
	return definitions
}

func collectReferences(object *astObject, file string, lines lineIndex) []Reference {
	var refs []Reference
	for _, member := range object.members {
		switch value := member.(type) {
		case astObjectSpread:
			refs = append(refs, reference(value.sub.path, ReferenceObjectSpread, file, lines))
		case astField:
			refs = append(refs, collectValueReferences(value.value, file, lines)...)
		}
	}
	return refs
}

func collectValueReferences(value astValue, file string, lines lineIndex) []Reference {
	switch v := value.(type) {
	case astSubstitution:
		return []Reference{reference(v.path, ReferenceSubstitution, file, lines)}
	case astString:
		var refs []Reference
		for _, part := range v.parts {
			if interpolation, ok := part.(stringInterpolationPart); ok {
				refs = append(refs, reference(interpolation.path, ReferenceInterpolation, file, lines))
			}
		}
		return refs
	case astArray:
		var refs []Reference
		for _, item := range v.items {
			switch item := item.(type) {
			case astArrayValue:
				refs = append(refs, collectValueReferences(item.value, file, lines)...)
			case astArraySpread:
				refs = append(refs, reference(item.sub.path, ReferenceArraySpread, file, lines))
			}
		}
		return refs
	case astObjectValue:
		return collectReferences(v.object, file, lines)
	default:
		return nil
	}
}

func collectIncludes(object *astObject, file string, lines lineIndex) []IncludeReference {
	var includes []IncludeReference
	for _, member := range object.members {
		switch value := member.(type) {
		case astInclude:
			includes = append(includes, IncludeReference{Path: value.path.value, File: file, Range: lines.rangeOf(value.span)})
		case astField:
			if nested, ok := value.value.(astObjectValue); ok {
				includes = append(includes, collectIncludes(nested.object, file, lines)...)
			}
		}
	}
	return includes
}

func reference(path astPath, kind ReferenceKind, file string, lines lineIndex) Reference {
	return Reference{Path: pathNames(path), Kind: kind, File: file, Range: lines.rangeOf(path.span)}
}

func resolveTargets(refs []Reference, definitions []Definition) {
	byPath := map[string]Definition{}
	for _, definition := range definitions {
		byPath[pathKey(definition.Path)] = definition
	}
	for idx := range refs {
		if target, ok := byPath[pathKey(refs[idx].Path)]; ok {
			refs[idx].Target = &target
		}
	}
}

func diagnosticFromError(err error, lines lineIndex, file string) Diagnostic {
	if sconErr, ok := err.(*Error); ok {
		var rng *SourceRange
		if sconErr.Span != nil {
			r := lines.rangeOf(*sconErr.Span)
			rng = &r
		}
		return Diagnostic{Code: sconErr.Code, Message: sconErr.Msg, Severity: DiagnosticError, File: file, Range: rng}
	}
	return Diagnostic{Code: Serde, Message: err.Error(), Severity: DiagnosticError, File: file}
}

func tokenInfos(tokens []token, lines lineIndex) []TokenInfo {
	out := make([]TokenInfo, 0, len(tokens))
	for _, token := range tokens {
		out = append(out, TokenInfo{Kind: token.kind.String(), Text: token.text, Span: token.span, Range: lines.rangeOf(token.span)})
	}
	return out
}

func comments(tokens []token, lines lineIndex) []Comment {
	var out []Comment
	for _, token := range tokens {
		if token.kind == tokComment {
			out = append(out, Comment{Text: token.text, Span: token.span, Range: lines.rangeOf(token.span)})
		}
	}
	return out
}

func appendPath(prefix []string, values ...string) []string {
	out := make([]string, 0, len(prefix)+len(values))
	out = append(out, prefix...)
	out = append(out, values...)
	return out
}

func pathKey(path []string) string {
	key := ""
	for _, segment := range path {
		key += "\x00" + segment
	}
	return key
}

func newLineIndex(source string) lineIndex {
	lines := []int{0}
	for idx, ch := range source {
		if ch == '\n' {
			lines = append(lines, idx+1)
		}
	}
	return lineIndex{source: source, lines: lines}
}

func (l lineIndex) rangeOf(span Span) SourceRange {
	return SourceRange{Start: l.position(span.Start), End: l.position(span.End), Span: span}
}

func (l lineIndex) position(offset int) SourcePosition {
	line := 0
	for line+1 < len(l.lines) && l.lines[line+1] <= offset {
		line++
	}
	return SourcePosition{Line: line, Column: offset - l.lines[line]}
}

func (k tokenKind) String() string {
	switch k {
	case tokIdentifier:
		return "identifier"
	case tokString:
		return "string"
	case tokNumber:
		return "number"
	case tokTrue:
		return "true"
	case tokFalse:
		return "false"
	case tokNull:
		return "null"
	case tokInclude:
		return "include"
	case tokSubstitutionStart:
		return "subst"
	case tokLeftBrace:
		return "{"
	case tokRightBrace:
		return "}"
	case tokLeftBracket:
		return "["
	case tokRightBracket:
		return "]"
	case tokEquals:
		return "="
	case tokDot:
		return "."
	case tokComma:
		return ","
	case tokSpread:
		return "..."
	case tokComment:
		return "comment"
	case tokNewline:
		return "newline"
	case tokWhitespace:
		return "ws"
	default:
		return "eof"
	}
}
