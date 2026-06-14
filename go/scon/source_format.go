package scon

import "strings"

func FormatSource(source string) (string, error) {
	doc, err := parseDocument(source, "")
	if err != nil {
		return "", err
	}
	var out strings.Builder
	if tokens, err := lex(source); err == nil {
		for _, token := range tokens {
			if token.kind == tokComment {
				out.WriteString(token.text)
				out.WriteByte('\n')
			}
		}
	}
	writeSourceObjectBody(&out, doc.root, 0)
	out.WriteByte('\n')
	return out.String(), nil
}

func writeSourceObjectBody(out *strings.Builder, object *astObject, indent int) {
	for _, member := range object.members {
		writeIndent(out, indent)
		switch value := member.(type) {
		case astInclude:
			out.WriteString("include ")
			out.WriteString(value.path.raw)
		case astObjectSpread:
			out.WriteString("...")
			writeSourceSubstitution(out, value.sub)
		case astField:
			writeSourcePath(out, value.path)
			out.WriteString(" = ")
			writeSourceValue(out, value.value, indent)
		}
		out.WriteByte('\n')
	}
}

func writeSourceValue(out *strings.Builder, value astValue, indent int) {
	switch v := value.(type) {
	case astNull:
		out.WriteString("null")
	case astBool:
		if v.value {
			out.WriteString("true")
		} else {
			out.WriteString("false")
		}
	case astNumber:
		out.WriteString(v.raw)
	case astString:
		out.WriteString(v.raw)
	case astSubstitution:
		writeSourceSubstitution(out, v)
	case astArray:
		if len(v.items) == 0 {
			out.WriteString("[]")
			return
		}
		out.WriteString("[\n")
		for _, item := range v.items {
			writeIndent(out, indent+2)
			switch item := item.(type) {
			case astArrayValue:
				writeSourceValue(out, item.value, indent+2)
			case astArraySpread:
				out.WriteString("...")
				writeSourceSubstitution(out, item.sub)
			}
			out.WriteString(",\n")
		}
		writeIndent(out, indent)
		out.WriteByte(']')
	case astObjectValue:
		if len(v.object.members) == 0 {
			out.WriteString("{}")
			return
		}
		out.WriteString("{\n")
		writeSourceObjectBody(out, v.object, indent+2)
		writeIndent(out, indent)
		out.WriteByte('}')
	}
}

func writeSourceSubstitution(out *strings.Builder, sub astSubstitution) {
	out.WriteString("${")
	writeSourcePath(out, sub.path)
	out.WriteByte('}')
}

func writeSourcePath(out *strings.Builder, path astPath) {
	for idx, segment := range path.segments {
		if idx > 0 {
			out.WriteByte('.')
		}
		if segment.quoted || !isSconIdentifier(segment.value) {
			writeString(out, segment.value)
		} else {
			out.WriteString(segment.value)
		}
	}
}
