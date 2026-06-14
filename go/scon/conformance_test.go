package scon

import (
	"bytes"
	"encoding/json"
	"math/big"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

type manifest struct {
	Cases []caseEntry `json:"cases"`
}

type caseEntry struct {
	ID          string   `json:"id"`
	Description string   `json:"description"`
	Entry       string   `json:"entry"`
	Kind        string   `json:"kind"`
	Expected    string   `json:"expected"`
	Tags        []string `json:"tags"`
}

func TestConformanceFixtures(t *testing.T) {
	root := filepath.Clean("../../tests/conformance")
	var manifest manifest
	readJSON(t, filepath.Join(root, "manifest.json"), &manifest)
	for _, c := range manifest.Cases {
		c := c
		t.Run(c.ID, func(t *testing.T) {
			entry := filepath.Join(root, c.Entry)
			expected := filepath.Join(root, c.Expected)
			switch c.Kind {
			case "valid":
				value, err := ParseFile(entry)
				if err != nil {
					t.Fatalf("valid case failed: %v\n%s", err, c.Description)
				}
				actualJSON := toJSONValue(value)
				var expectedJSON any
				readJSON(t, expected, &expectedJSON)
				if !jsonEqual(actualJSON, expectedJSON) {
					t.Fatalf("resolved differently\nactual: %#v\nexpected: %#v", actualJSON, expectedJSON)
				}
			case "invalid":
				_, err := ParseFile(entry)
				if err == nil {
					t.Fatalf("invalid case unexpectedly succeeded\n%s", c.Description)
				}
				var expectedError struct {
					Code string `json:"code"`
				}
				readJSON(t, expected, &expectedError)
				var sconErr *Error
				if !errorAs(err, &sconErr) {
					t.Fatalf("expected SCON error, got %T: %v", err, err)
				}
				if string(sconErr.Code) != expectedError.Code {
					t.Fatalf("wrong error code: got %s want %s: %v", sconErr.Code, expectedError.Code, err)
				}
			}
		})
	}
}

func jsonEqual(a, b any) bool {
	switch av := a.(type) {
	case json.Number:
		bv, ok := b.(json.Number)
		return ok && numberEqual(av.String(), bv.String())
	case map[string]any:
		bv, ok := b.(map[string]any)
		if !ok || len(av) != len(bv) {
			return false
		}
		for key, value := range av {
			if !jsonEqual(value, bv[key]) {
				return false
			}
		}
		return true
	case []any:
		bv, ok := b.([]any)
		if !ok || len(av) != len(bv) {
			return false
		}
		for i := range av {
			if !jsonEqual(av[i], bv[i]) {
				return false
			}
		}
		return true
	default:
		return reflect.DeepEqual(a, b)
	}
}

func numberEqual(a, b string) bool {
	af, _, errA := big.ParseFloat(a, 10, 256, big.ToNearestEven)
	bf, _, errB := big.ParseFloat(b, 10, 256, big.ToNearestEven)
	return errA == nil && errB == nil && af.Cmp(bf) == 0
}

func TestTypedRoundTrip(t *testing.T) {
	type Server struct {
		Host string `scon:"host"`
		Port uint16 `scon:"port"`
	}
	type Config struct {
		Name    string            `scon:"name"`
		Enabled bool              `scon:"enabled"`
		Server  Server            `scon:"server"`
		Tags    []string          `scon:"tags"`
		Meta    map[string]string `scon:"meta"`
	}
	cfg := Config{
		Name:    "demo",
		Enabled: true,
		Server:  Server{Host: "127.0.0.1", Port: 8080},
		Tags:    []string{"api", "prod"},
		Meta:    map[string]string{"region": "us"},
	}
	text, err := Marshal(cfg)
	if err != nil {
		t.Fatal(err)
	}
	var decoded Config
	if err := Unmarshal(text, &decoded); err != nil {
		t.Fatal(err)
	}
	if !reflect.DeepEqual(cfg, decoded) {
		t.Fatalf("round trip mismatch: %#v", decoded)
	}
}

func TestAnalysisAndFormatSource(t *testing.T) {
	source := "defaults { port = 8080 }\nserver = ${defaults.port}\nitems = [1, ...${extra}]\n"
	analysis := AnalyzeSource(source)
	if len(analysis.Diagnostics) != 1 || analysis.Diagnostics[0].Code != MissingReference {
		t.Fatalf("expected missing reference diagnostic, got %#v", analysis.Diagnostics)
	}
	if len(analysis.Symbols) < 3 {
		t.Fatalf("expected symbols, got %#v", analysis.Symbols)
	}
	if len(analysis.References) != 2 {
		t.Fatalf("expected references, got %#v", analysis.References)
	}

	formatted, err := FormatSource(`# keep me
include "base.scon"
defaults { port = 8080 }
server = ${defaults.port}
items = [1, ...${extra}]
`)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := parseDocument(formatted, ""); err != nil {
		t.Fatalf("formatted source did not parse: %v\n%s", err, formatted)
	}
	if !strings.Contains(formatted, `# keep me`) || !strings.Contains(formatted, `include "base.scon"`) || !strings.Contains(formatted, "...${extra}") {
		t.Fatalf("formatted source lost source-level constructs:\n%s", formatted)
	}
}

func readJSON(t *testing.T, path string, out any) {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	decoder := json.NewDecoder(bytes.NewReader(data))
	decoder.UseNumber()
	if err := decoder.Decode(out); err != nil {
		t.Fatal(err)
	}
}

func toJSONValue(value Value) any {
	switch v := value.(type) {
	case Null:
		return nil
	case Bool:
		return bool(v)
	case Number:
		return json.Number(v.String())
	case String:
		return string(v)
	case Array:
		out := make([]any, len(v))
		for i, item := range v {
			out[i] = toJSONValue(item)
		}
		return out
	case *Object:
		out := map[string]any{}
		for _, entry := range v.entries {
			out[entry.Key] = toJSONValue(entry.Value)
		}
		return out
	default:
		return v
	}
}

func errorAs(err error, target any) bool {
	switch t := target.(type) {
	case **Error:
		if e, ok := err.(*Error); ok {
			*t = e
			return true
		}
	}
	return false
}
