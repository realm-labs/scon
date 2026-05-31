package scon

import (
	"math"
	"strconv"
	"strings"
)

type NumberKind int

const (
	NumberI64 NumberKind = iota
	NumberU64
	NumberF64
)

type Number struct {
	kind NumberKind
	i64  int64
	u64  uint64
	f64  float64
}

func NewI64(v int64) Number  { return Number{kind: NumberI64, i64: v} }
func NewU64(v uint64) Number { return Number{kind: NumberU64, u64: v} }
func NewF64(v float64) (Number, error) {
	if math.IsNaN(v) || math.IsInf(v, 0) {
		return Number{}, simpleError(InvalidNumber, "invalid SCON number")
	}
	return Number{kind: NumberF64, f64: v}, nil
}

func ParseNumber(raw string) (Number, error) {
	if strings.ContainsAny(raw, ".eE") {
		v, err := strconv.ParseFloat(raw, 64)
		if err != nil || math.IsNaN(v) || math.IsInf(v, 0) {
			return Number{}, simpleError(InvalidNumber, "invalid SCON number: "+raw)
		}
		return Number{kind: NumberF64, f64: v}, nil
	}
	if strings.HasPrefix(raw, "-") {
		v, err := strconv.ParseInt(raw, 10, 64)
		if err != nil {
			return Number{}, simpleError(InvalidNumber, "invalid SCON number: "+raw)
		}
		return NewI64(v), nil
	}
	v, err := strconv.ParseUint(raw, 10, 64)
	if err != nil {
		return Number{}, simpleError(InvalidNumber, "invalid SCON number: "+raw)
	}
	return NewU64(v), nil
}

func (n Number) Kind() NumberKind { return n.kind }
func (n Number) Int64() (int64, bool) {
	switch n.kind {
	case NumberI64:
		return n.i64, true
	case NumberU64:
		if n.u64 <= math.MaxInt64 {
			return int64(n.u64), true
		}
	}
	return 0, false
}
func (n Number) Uint64() (uint64, bool) {
	switch n.kind {
	case NumberU64:
		return n.u64, true
	case NumberI64:
		if n.i64 >= 0 {
			return uint64(n.i64), true
		}
	}
	return 0, false
}
func (n Number) Float64() float64 {
	switch n.kind {
	case NumberI64:
		return float64(n.i64)
	case NumberU64:
		return float64(n.u64)
	default:
		return n.f64
	}
}
func (n Number) String() string {
	switch n.kind {
	case NumberI64:
		return strconv.FormatInt(n.i64, 10)
	case NumberU64:
		return strconv.FormatUint(n.u64, 10)
	default:
		return strconv.FormatFloat(n.f64, 'g', -1, 64)
	}
}

func numberFromReflectSigned(v int64) Number    { return NewI64(v) }
func numberFromReflectUnsigned(v uint64) Number { return NewU64(v) }
func numberFromReflectFloat(v float64) (Number, error) {
	return NewF64(v)
}
