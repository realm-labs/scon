package scon

import (
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"unicode/utf8"
)

func FuzzParseString(f *testing.F) {
	addConformanceSeeds(f)
	f.Fuzz(func(t *testing.T, source string) {
		if !utf8.ValidString(source) {
			t.Skip()
		}
		_, _ = ParseString(source)
	})
}

func FuzzFormatSource(f *testing.F) {
	addConformanceSeeds(f)
	f.Fuzz(func(t *testing.T, source string) {
		if !utf8.ValidString(source) {
			t.Skip()
		}
		formatted, err := FormatSource(source)
		if err != nil {
			return
		}
		if _, err := parseDocument(formatted, ""); err != nil {
			t.Fatalf("formatted source does not parse: %v\n%s", err, formatted)
		}
		originalValue, originalErr := ParseString(source)
		formattedValue, formattedErr := ParseString(formatted)
		if originalErr == nil && formattedErr == nil && !reflect.DeepEqual(originalValue, formattedValue) {
			t.Fatalf("formatted source changed resolved value")
		}
	})
}

func addConformanceSeeds(f *testing.F) {
	f.Add("")
	f.Add("# comment\n")
	f.Add("name = \"demo\"\n")
	root := filepath.Clean("../../tests/conformance")
	_ = filepath.WalkDir(root, func(path string, entry os.DirEntry, err error) error {
		if err != nil || entry.IsDir() || !strings.HasSuffix(entry.Name(), ".scon") {
			return nil
		}
		data, err := os.ReadFile(path)
		if err == nil && utf8.Valid(data) {
			f.Add(string(data))
		}
		return nil
	})
}
