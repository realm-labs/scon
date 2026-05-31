package scon

import (
	"strconv"
	"strings"
)

type document struct {
	root *astObject
	file string
}

type astObject struct {
	members []astMember
	span    Span
}

type astMember interface {
	memberSpan() Span
}

type astField struct {
	path  astPath
	value astValue
	span  Span
}
type astInclude struct {
	path astString
	span Span
}
type astObjectSpread struct {
	sub  astSubstitution
	span Span
}

func (m astField) memberSpan() Span        { return m.span }
func (m astInclude) memberSpan() Span      { return m.span }
func (m astObjectSpread) memberSpan() Span { return m.span }

type astPath struct {
	segments []astPathSegment
	span     Span
}
type astPathSegment struct {
	value  string
	quoted bool
	span   Span
}

type astValue interface {
	valueSpan() Span
}

type astNull struct{ span Span }
type astBool struct {
	value bool
	span  Span
}
type astNumber struct {
	raw  string
	span Span
}
type astString struct {
	value string
	raw   string
	parts []stringPart
	span  Span
}
type astArray struct {
	items []astArrayItem
	span  Span
}
type astObjectValue struct{ object *astObject }
type astSubstitution struct {
	path astPath
	span Span
}

func (v astNull) valueSpan() Span         { return v.span }
func (v astBool) valueSpan() Span         { return v.span }
func (v astNumber) valueSpan() Span       { return v.span }
func (v astString) valueSpan() Span       { return v.span }
func (v astArray) valueSpan() Span        { return v.span }
func (v astObjectValue) valueSpan() Span  { return v.object.span }
func (v astSubstitution) valueSpan() Span { return v.span }

type astArrayItem interface {
	itemSpan() Span
}
type astArrayValue struct {
	value astValue
	span  Span
}
type astArraySpread struct {
	sub  astSubstitution
	span Span
}

func (i astArrayValue) itemSpan() Span  { return i.span }
func (i astArraySpread) itemSpan() Span { return i.span }

type stringPart interface {
	stringPart()
}
type stringLiteralPart struct{ value string }
type stringInterpolationPart struct {
	path astPath
	span Span
}

func (stringLiteralPart) stringPart()       {}
func (stringInterpolationPart) stringPart() {}

type parser struct {
	tokens []token
	index  int
}

func parseDocument(source, file string) (*document, error) {
	tokens, err := lex(source)
	if err != nil {
		return nil, err
	}
	p := &parser{tokens: tokens}
	p.skipTrivia()
	var root *astObject
	if p.match(tokLeftBrace) {
		root, err = p.parseObject(p.previous())
	} else if p.check(tokLeftBracket) {
		return nil, sconError(InvalidRootType, "SCON document root must be an object", p.peek().span)
	} else {
		root, err = p.parseObjectBody(p.peek().span.Start)
	}
	if err != nil {
		return nil, err
	}
	p.skipTrivia()
	if _, err := p.expect(tokEOF, "expected end of file"); err != nil {
		return nil, err
	}
	return &document{root: root, file: file}, nil
}

func (p *parser) parseObject(opening token) (*astObject, error) {
	var members []astMember
	p.skipTrivia()
	for !p.check(tokRightBrace) && !p.check(tokEOF) {
		member, err := p.parseObjectMember()
		if err != nil {
			return nil, err
		}
		members = append(members, member)
		p.skipTrivia()
		if p.match(tokComma) {
			p.skipTrivia()
			if p.check(tokComma) {
				return nil, sconError(UnexpectedToken, "consecutive commas are invalid", p.peek().span)
			}
		}
	}
	closing, err := p.expect(tokRightBrace, "expected '}'")
	if err != nil {
		return nil, err
	}
	return &astObject{members: members, span: Span{opening.span.Start, closing.span.End}}, nil
}

func (p *parser) parseObjectBody(start int) (*astObject, error) {
	var members []astMember
	p.skipTrivia()
	for !p.check(tokEOF) && !p.check(tokRightBrace) {
		member, err := p.parseObjectMember()
		if err != nil {
			return nil, err
		}
		members = append(members, member)
		p.skipTrivia()
		if p.match(tokComma) {
			p.skipTrivia()
			if p.check(tokComma) {
				return nil, sconError(UnexpectedToken, "consecutive commas are invalid", p.peek().span)
			}
		}
	}
	end := start
	if len(members) > 0 {
		end = members[len(members)-1].memberSpan().End
	}
	return &astObject{members: members, span: Span{start, end}}, nil
}

func (p *parser) parseObjectMember() (astMember, error) {
	p.skipTrivia()
	if p.match(tokInclude) {
		inc := p.previous()
		p.skipInlineTrivia()
		path, err := p.parseString()
		if err != nil {
			return nil, err
		}
		for _, part := range path.parts {
			if _, ok := part.(stringInterpolationPart); ok {
				return nil, sconError(UnexpectedToken, "include path must be a literal string", path.span)
			}
		}
		return astInclude{path: path, span: Span{inc.span.Start, path.span.End}}, nil
	}
	if p.match(tokSpread) {
		spread := p.previous()
		sub, err := p.parseSubstitution()
		if err != nil {
			return nil, err
		}
		return astObjectSpread{sub: sub, span: Span{spread.span.Start, sub.span.End}}, nil
	}
	path, err := p.parsePath()
	if err != nil {
		return nil, err
	}
	p.skipInlineTrivia()
	var value astValue
	if p.match(tokEquals) {
		p.skipInlineTrivia()
		if p.check(tokNewline) {
			return nil, sconError(UnexpectedToken, "field value cannot start on the next line", p.peek().span)
		}
		value, err = p.parseValue()
	} else if p.match(tokLeftBrace) {
		var obj *astObject
		obj, err = p.parseObject(p.previous())
		value = astObjectValue{object: obj}
	} else {
		return nil, sconError(UnexpectedToken, "expected '=' or object shorthand", p.peek().span)
	}
	if err != nil {
		return nil, err
	}
	return astField{path: path, value: value, span: Span{path.span.Start, value.valueSpan().End}}, nil
}

func (p *parser) parseValue() (astValue, error) {
	p.skipTrivia()
	switch {
	case p.match(tokNull):
		return astNull{span: p.previous().span}, nil
	case p.match(tokTrue):
		return astBool{value: true, span: p.previous().span}, nil
	case p.match(tokFalse):
		return astBool{value: false, span: p.previous().span}, nil
	case p.match(tokNumber):
		tok := p.previous()
		return astNumber{raw: tok.text, span: tok.span}, nil
	case p.check(tokString):
		return p.parseString()
	case p.match(tokLeftBrace):
		obj, err := p.parseObject(p.previous())
		if err != nil {
			return nil, err
		}
		return astObjectValue{object: obj}, nil
	case p.match(tokLeftBracket):
		return p.parseArray(p.previous())
	case p.check(tokSubstitutionStart):
		return p.parseSubstitution()
	default:
		return nil, sconError(UnexpectedToken, "expected value", p.peek().span)
	}
}

func (p *parser) parseArray(opening token) (astArray, error) {
	var items []astArrayItem
	p.skipTrivia()
	for !p.check(tokRightBracket) && !p.check(tokEOF) {
		itemStart := p.peek().span.Start
		if p.match(tokSpread) {
			sub, err := p.parseSubstitution()
			if err != nil {
				return astArray{}, err
			}
			items = append(items, astArraySpread{sub: sub, span: Span{itemStart, sub.span.End}})
		} else {
			value, err := p.parseValue()
			if err != nil {
				return astArray{}, err
			}
			items = append(items, astArrayValue{value: value, span: value.valueSpan()})
		}
		p.skipTrivia()
		if !p.match(tokComma) {
			break
		}
		p.skipTrivia()
		if p.check(tokComma) {
			return astArray{}, sconError(UnexpectedToken, "consecutive commas are invalid", p.peek().span)
		}
	}
	closing, err := p.expect(tokRightBracket, "expected ']'")
	if err != nil {
		return astArray{}, err
	}
	return astArray{items: items, span: Span{opening.span.Start, closing.span.End}}, nil
}

func (p *parser) parseSubstitution() (astSubstitution, error) {
	start, err := p.expect(tokSubstitutionStart, "expected '${'")
	if err != nil {
		return astSubstitution{}, err
	}
	path, err := p.parsePath()
	if err != nil {
		return astSubstitution{}, err
	}
	end, err := p.expect(tokRightBrace, "expected '}'")
	if err != nil {
		return astSubstitution{}, err
	}
	return astSubstitution{path: path, span: Span{start.span.Start, end.span.End}}, nil
}

func (p *parser) parsePath() (astPath, error) {
	first, err := p.parsePathSegment()
	if err != nil {
		return astPath{}, err
	}
	segments := []astPathSegment{first}
	for p.match(tokDot) {
		seg, err := p.parsePathSegment()
		if err != nil {
			return astPath{}, err
		}
		segments = append(segments, seg)
	}
	return astPath{segments: segments, span: Span{first.span.Start, segments[len(segments)-1].span.End}}, nil
}

func (p *parser) parsePathSegment() (astPathSegment, error) {
	if p.match(tokIdentifier) {
		tok := p.previous()
		return astPathSegment{value: tok.text, span: tok.span}, nil
	}
	if p.check(tokString) {
		str, err := p.parseString()
		if err != nil {
			return astPathSegment{}, err
		}
		return astPathSegment{value: str.value, quoted: true, span: str.span}, nil
	}
	return astPathSegment{}, sconError(UnexpectedToken, "expected path segment", p.peek().span)
}

func (p *parser) parseString() (astString, error) {
	tok, err := p.expect(tokString, "expected string")
	if err != nil {
		return astString{}, err
	}
	parts, value, err := parseStringParts(tok)
	if err != nil {
		return astString{}, err
	}
	return astString{value: value, raw: tok.text, parts: parts, span: tok.span}, nil
}

func parseStringParts(tok token) ([]stringPart, string, error) {
	raw := tok.text
	var parts []stringPart
	var out strings.Builder
	var value strings.Builder
	for i := 1; i < len(raw)-1; {
		ch := raw[i]
		i++
		if ch == '$' && i < len(raw)-1 && raw[i] == '{' {
			if out.Len() > 0 {
				text := out.String()
				parts = append(parts, stringLiteralPart{text})
				value.WriteString(text)
				out.Reset()
			}
			pathStart := i + 1
			close := strings.IndexByte(raw[pathStart:], '}')
			if close < 0 {
				return nil, "", sconError(UnterminatedString, "unterminated interpolation", tok.span)
			}
			close += pathStart
			path, err := parseInterpolationPath(raw[pathStart:close], tok.span.Start+pathStart)
			if err != nil {
				return nil, "", err
			}
			parts = append(parts, stringInterpolationPart{path: path, span: Span{tok.span.Start + i - 1, tok.span.Start + close + 1}})
			i = close + 1
			continue
		}
		if ch != '\\' {
			out.WriteByte(ch)
			continue
		}
		if i >= len(raw)-1 {
			return nil, "", sconError(UnterminatedString, "unterminated string escape", tok.span)
		}
		esc := raw[i]
		i++
		switch esc {
		case '"':
			out.WriteByte('"')
		case '\\':
			out.WriteByte('\\')
		case '/':
			out.WriteByte('/')
		case 'b':
			out.WriteByte('\b')
		case 'f':
			out.WriteByte('\f')
		case 'n':
			out.WriteByte('\n')
		case 'r':
			out.WriteByte('\r')
		case 't':
			out.WriteByte('\t')
		case '$':
			out.WriteByte('$')
		case 'u':
			if i+4 > len(raw)-1 {
				return nil, "", sconError(InvalidEscape, "invalid unicode escape", tok.span)
			}
			code, err := strconv.ParseInt(raw[i:i+4], 16, 32)
			if err != nil {
				return nil, "", sconError(InvalidEscape, "invalid unicode escape", tok.span)
			}
			out.WriteRune(rune(code))
			i += 4
		default:
			return nil, "", sconError(InvalidEscape, "invalid string escape", tok.span)
		}
	}
	if out.Len() > 0 || len(parts) == 0 {
		text := out.String()
		parts = append(parts, stringLiteralPart{text})
		value.WriteString(text)
	}
	return parts, value.String(), nil
}

func parseInterpolationPath(text string, base int) (astPath, error) {
	if strings.HasPrefix(text, ".") || strings.HasPrefix(text, "?") || strings.Contains(text, ":-") {
		return astPath{}, sconError(UnexpectedToken, "invalid substitution path", Span{base, max(base+1, base+len(text))})
	}
	tokens, err := lex(text)
	if err != nil {
		return astPath{}, err
	}
	p := &parser{tokens: tokens}
	path, err := p.parsePath()
	if err != nil {
		return astPath{}, err
	}
	if _, err := p.expect(tokEOF, "expected end of substitution path"); err != nil {
		return astPath{}, err
	}
	for i := range path.segments {
		path.segments[i].span.Start += base
		path.segments[i].span.End += base
	}
	path.span.Start += base
	path.span.End += base
	return path, nil
}

func (p *parser) skipTrivia() {
	for p.match(tokWhitespace) || p.match(tokNewline) || p.match(tokComment) {
	}
}
func (p *parser) skipInlineTrivia() {
	for p.match(tokWhitespace) || p.match(tokComment) {
	}
}
func (p *parser) match(kind tokenKind) bool {
	if !p.check(kind) {
		return false
	}
	p.index++
	return true
}
func (p *parser) check(kind tokenKind) bool {
	return p.peek().kind == kind
}
func (p *parser) expect(kind tokenKind, message string) (token, error) {
	if p.check(kind) {
		p.index++
		return p.previous(), nil
	}
	return token{}, sconError(UnexpectedToken, message, p.peek().span)
}
func (p *parser) peek() token {
	if p.index >= len(p.tokens) {
		return p.tokens[len(p.tokens)-1]
	}
	return p.tokens[p.index]
}
func (p *parser) previous() token { return p.tokens[p.index-1] }
