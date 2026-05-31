package scon

import "fmt"

type ErrorCode string

const (
	InvalidCharacter      ErrorCode = "InvalidCharacter"
	InvalidWhitespace     ErrorCode = "InvalidWhitespace"
	InvalidEscape         ErrorCode = "InvalidEscape"
	UnexpectedToken       ErrorCode = "UnexpectedToken"
	UnterminatedString    ErrorCode = "UnterminatedString"
	InvalidNumber         ErrorCode = "InvalidNumber"
	InvalidRootType       ErrorCode = "InvalidRootType"
	DuplicateKey          ErrorCode = "DuplicateKey"
	PathConflict          ErrorCode = "PathConflict"
	MissingReference      ErrorCode = "MissingReference"
	TypeMismatch          ErrorCode = "TypeMismatch"
	InvalidSpread         ErrorCode = "InvalidSpread"
	InvalidIncludePath    ErrorCode = "InvalidIncludePath"
	IncludeNotFound       ErrorCode = "IncludeNotFound"
	IncludeNotFile        ErrorCode = "IncludeNotFile"
	IncludePathDenied     ErrorCode = "IncludePathDenied"
	IncludeCycle          ErrorCode = "IncludeCycle"
	IncludeParseError     ErrorCode = "IncludeParseError"
	IncludeRootTypeError  ErrorCode = "IncludeRootTypeError"
	ResourceLimitExceeded ErrorCode = "ResourceLimitExceeded"
	Serde                 ErrorCode = "Serde"
)

type Span struct {
	Start int
	End   int
}

type Error struct {
	Code ErrorCode
	Msg  string
	Span *Span
}

func (e *Error) Error() string {
	if e == nil {
		return "<nil>"
	}
	return fmt.Sprintf("%s: %s", e.Code, e.Msg)
}

func sconError(code ErrorCode, msg string, span Span) *Error {
	return &Error{Code: code, Msg: msg, Span: &span}
}

func simpleError(code ErrorCode, msg string) *Error {
	return &Error{Code: code, Msg: msg}
}
