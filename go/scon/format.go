package scon

import (
	"fmt"
	"strings"
	"unicode"
)

func FormatValue(value Value) (string, error) {
	object, ok := value.(*Object)
	if !ok {
		return "", simpleError(InvalidRootType, "SCON document root must be an object")
	}
	var out strings.Builder
	writeObjectBody(&out, object, 0)
	out.WriteByte('\n')
	return out.String(), nil
}

func writeObjectBody(out *strings.Builder, object *Object, indent int) {
	for _, entry := range object.entries {
		writeIndent(out, indent)
		out.WriteString(formatKey(entry.Key))
		out.WriteString(" = ")
		writeValue(out, entry.Value, indent)
		out.WriteByte('\n')
	}
}

func writeValue(out *strings.Builder, value Value, indent int) {
	switch v := value.(type) {
	case Null:
		out.WriteString("null")
	case Bool:
		if bool(v) {
			out.WriteString("true")
		} else {
			out.WriteString("false")
		}
	case Number:
		out.WriteString(v.String())
	case String:
		writeString(out, string(v))
	case Array:
		if len(v) == 0 {
			out.WriteString("[]")
			return
		}
		out.WriteString("[\n")
		for _, item := range v {
			writeIndent(out, indent+2)
			writeValue(out, item, indent+2)
			out.WriteString(",\n")
		}
		writeIndent(out, indent)
		out.WriteByte(']')
	case *Object:
		if v.Len() == 0 {
			out.WriteString("{}")
			return
		}
		out.WriteString("{\n")
		writeObjectBody(out, v, indent+2)
		writeIndent(out, indent)
		out.WriteByte('}')
	}
}

func writeIndent(out *strings.Builder, indent int) {
	for range indent {
		out.WriteByte(' ')
	}
}

func writeString(out *strings.Builder, value string) {
	out.WriteByte('"')
	for i, r := range value {
		switch r {
		case '"':
			out.WriteString("\\\"")
		case '\\':
			out.WriteString("\\\\")
		case '\n':
			out.WriteString("\\n")
		case '\r':
			out.WriteString("\\r")
		case '\t':
			out.WriteString("\\t")
		case '\b':
			out.WriteString("\\b")
		case '\f':
			out.WriteString("\\f")
		case '$':
			if i+1 < len(value) && value[i+1] == '{' {
				out.WriteString("\\$")
			} else {
				out.WriteRune(r)
			}
		default:
			if unicode.IsControl(r) {
				out.WriteString(fmt.Sprintf("\\u%04X", r))
			} else {
				out.WriteRune(r)
			}
		}
	}
	out.WriteByte('"')
}

func formatKey(key string) string {
	if isSconIdentifier(key) {
		return key
	}
	var out strings.Builder
	writeString(&out, key)
	return out.String()
}

func isSconIdentifier(value string) bool {
	if isReservedIdentifier(value) {
		return false
	}
	if value == "" {
		return false
	}
	for i, ch := range value {
		if i == 0 {
			if !((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || ch == '_') {
				return false
			}
			continue
		}
		if !((ch >= 'a' && ch <= 'z') || (ch >= 'A' && ch <= 'Z') || (ch >= '0' && ch <= '9') || ch == '_' || ch == '-') {
			return false
		}
	}
	return true
}

func isReservedIdentifier(value string) bool {
	return value == "include" || value == "true" || value == "false" || value == "null"
}

func GetPath(value Value, path string) (Value, error) {
	current := value
	for _, segment := range strings.Split(path, ".") {
		object, ok := current.(*Object)
		if !ok {
			return nil, simpleError(TypeMismatch, "path segment requires object")
		}
		next, ok := object.Get(segment)
		if !ok {
			return nil, simpleError(MissingReference, "path is not defined")
		}
		current = next
	}
	return current, nil
}
