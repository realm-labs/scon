package scon

import (
	"unicode"
	"unicode/utf8"
)

type tokenKind int

const (
	tokIdentifier tokenKind = iota
	tokString
	tokNumber
	tokTrue
	tokFalse
	tokNull
	tokInclude
	tokSubstitutionStart
	tokLeftBrace
	tokRightBrace
	tokLeftBracket
	tokRightBracket
	tokEquals
	tokDot
	tokComma
	tokSpread
	tokComment
	tokNewline
	tokWhitespace
	tokEOF
)

type token struct {
	kind tokenKind
	text string
	span Span
}

type lexer struct {
	source string
	tokens []token
	index  int
}

func lex(source string) ([]token, error) {
	l := &lexer{source: source}
	for !l.atEnd() {
		start := l.index
		ch := l.source[l.index]
		switch ch {
		case ' ', '\t':
			l.lexHorizontalWhitespace()
		case '\n':
			l.index++
			l.add(tokNewline, start, l.index)
		case '\r':
			if l.peek(1) == '\n' {
				l.index += 2
				l.add(tokNewline, start, l.index)
			} else {
				return nil, sconError(InvalidCharacter, "standalone CR is invalid", Span{start, min(start+1, len(l.source))})
			}
		case '#':
			l.lexLineComment()
		case '/':
			if l.peek(1) != '/' {
				return nil, sconError(InvalidCharacter, "unexpected character '/'", Span{start, start + 1})
			}
			l.index += 2
			for !l.atEnd() && l.source[l.index] != '\n' && l.source[l.index] != '\r' {
				l.index++
			}
			l.add(tokComment, start, l.index)
		case '"':
			if err := l.lexString(); err != nil {
				return nil, err
			}
		case '$':
			if l.peek(1) != '{' {
				return nil, sconError(InvalidCharacter, "unexpected character '$'", Span{start, start + 1})
			}
			l.index += 2
			l.add(tokSubstitutionStart, start, l.index)
		case '{':
			l.index++
			l.add(tokLeftBrace, start, l.index)
		case '}':
			l.index++
			l.add(tokRightBrace, start, l.index)
		case '[':
			l.index++
			l.add(tokLeftBracket, start, l.index)
		case ']':
			l.index++
			l.add(tokRightBracket, start, l.index)
		case '=':
			l.index++
			l.add(tokEquals, start, l.index)
		case '.':
			if l.peek(1) == '.' && l.peek(2) == '.' {
				l.index += 3
				l.add(tokSpread, start, l.index)
			} else {
				l.index++
				l.add(tokDot, start, l.index)
			}
		case ',':
			l.index++
			l.add(tokComma, start, l.index)
		case '-':
			if !isASCIIDigit(l.peek(1)) {
				return nil, sconError(UnexpectedToken, "expected digit after '-'", Span{start, start + 1})
			}
			if err := l.lexNumber(); err != nil {
				return nil, err
			}
		case '?', ':':
			return nil, sconError(UnexpectedToken, "unexpected character", Span{start, start + 1})
		default:
			switch {
			case isASCIIDigit(ch):
				if err := l.lexNumber(); err != nil {
					return nil, err
				}
			case isIdentifierStart(ch):
				l.lexIdentifier()
			case ch >= utf8.RuneSelf:
				r, size := utf8.DecodeRuneInString(l.source[l.index:])
				if unicode.IsSpace(r) {
					return nil, sconError(InvalidWhitespace, "invalid whitespace outside strings", Span{start, start + size})
				}
				return nil, sconError(InvalidCharacter, "unexpected character", Span{start, start + size})
			default:
				return nil, sconError(InvalidCharacter, "unexpected character", Span{start, start + 1})
			}
		}
	}
	l.tokens = append(l.tokens, token{kind: tokEOF, span: Span{len(l.source), len(l.source)}})
	return l.tokens, nil
}

func (l *lexer) lexHorizontalWhitespace() {
	start := l.index
	for !l.atEnd() && (l.source[l.index] == ' ' || l.source[l.index] == '\t') {
		l.index++
	}
	l.add(tokWhitespace, start, l.index)
}

func (l *lexer) lexLineComment() {
	start := l.index
	l.index++
	for !l.atEnd() && l.source[l.index] != '\n' && l.source[l.index] != '\r' {
		l.index++
	}
	l.add(tokComment, start, l.index)
}

func (l *lexer) lexIdentifier() {
	start := l.index
	l.index++
	for !l.atEnd() && isIdentifierPart(l.source[l.index]) {
		l.index++
	}
	text := l.source[start:l.index]
	kind := tokIdentifier
	switch text {
	case "true":
		kind = tokTrue
	case "false":
		kind = tokFalse
	case "null":
		kind = tokNull
	case "include":
		kind = tokInclude
	}
	l.add(kind, start, l.index)
}

func (l *lexer) lexNumber() error {
	start := l.index
	if l.source[l.index] == '-' {
		l.index++
	}
	if l.peek(0) == '0' {
		l.index++
		if isASCIIDigit(l.peek(0)) {
			return sconError(InvalidNumber, "leading zeroes are invalid", Span{start, l.index})
		}
	} else {
		if !isASCIIDigitNonZero(l.peek(0)) {
			return sconError(InvalidNumber, "invalid number", Span{start, l.index})
		}
		for isASCIIDigit(l.peek(0)) {
			l.index++
		}
	}
	if l.peek(0) == '.' {
		l.index++
		if !isASCIIDigit(l.peek(0)) {
			return sconError(InvalidNumber, "expected digit after decimal point", Span{start, l.index})
		}
		for isASCIIDigit(l.peek(0)) {
			l.index++
		}
	}
	if l.peek(0) == 'e' || l.peek(0) == 'E' {
		l.index++
		if l.peek(0) == '+' || l.peek(0) == '-' {
			l.index++
		}
		if !isASCIIDigit(l.peek(0)) {
			return sconError(InvalidNumber, "expected exponent digit", Span{start, l.index})
		}
		for isASCIIDigit(l.peek(0)) {
			l.index++
		}
	}
	l.add(tokNumber, start, l.index)
	return nil
}

func (l *lexer) lexString() error {
	start := l.index
	l.index++
	for !l.atEnd() {
		switch l.source[l.index] {
		case '"':
			l.index++
			l.add(tokString, start, l.index)
			return nil
		case '\n', '\r':
			return sconError(UnterminatedString, "raw multiline strings are invalid", Span{l.index, l.index + 1})
		case '\\':
			l.index++
			if l.atEnd() {
				return sconError(UnterminatedString, "unterminated string escape", Span{l.index, l.index})
			}
			switch l.source[l.index] {
			case '"', '\\', '/', 'b', 'f', 'n', 'r', 't', '$':
				l.index++
			case 'u':
				l.index++
				for range 4 {
					if !isHex(l.peek(0)) {
						return sconError(InvalidEscape, "invalid unicode escape", Span{l.index, min(l.index+1, len(l.source))})
					}
					l.index++
				}
			default:
				return sconError(InvalidEscape, "invalid string escape", Span{l.index - 1, l.index})
			}
		default:
			l.index++
		}
	}
	return sconError(UnterminatedString, "unterminated string", Span{start, len(l.source)})
}

func (l *lexer) add(kind tokenKind, start, end int) {
	l.tokens = append(l.tokens, token{kind: kind, text: l.source[start:end], span: Span{start, end}})
}

func (l *lexer) peek(offset int) byte {
	if l.index+offset >= len(l.source) {
		return 0
	}
	return l.source[l.index+offset]
}

func (l *lexer) atEnd() bool { return l.index >= len(l.source) }

func isASCIIDigit(ch byte) bool        { return ch >= '0' && ch <= '9' }
func isASCIIDigitNonZero(ch byte) bool { return ch >= '1' && ch <= '9' }
func isHex(ch byte) bool {
	return (ch >= '0' && ch <= '9') || (ch >= 'a' && ch <= 'f') || (ch >= 'A' && ch <= 'F')
}
func isIdentifierStart(ch byte) bool {
	return (ch >= 'A' && ch <= 'Z') || (ch >= 'a' && ch <= 'z') || ch == '_'
}
func isIdentifierPart(ch byte) bool { return isIdentifierStart(ch) || isASCIIDigit(ch) || ch == '-' }
