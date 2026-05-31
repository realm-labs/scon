package scon

import (
	"os"
	"path/filepath"
	"runtime"
	"strings"
)

type LoadOptions struct {
	IncludeRoot string
	Limits      Limits
}

type LoadOption func(*LoadOptions)

type Limits struct {
	MaxFileSize     int
	MaxIncludeDepth int
	MaxIncludeFiles int
	MaxArrayLength  int
	MaxObjectDepth  int
}

func defaultLimits() Limits {
	return Limits{
		MaxFileSize:     16 * 1024 * 1024,
		MaxIncludeDepth: 64,
		MaxIncludeFiles: 1024,
		MaxArrayLength:  1_000_000,
		MaxObjectDepth:  512,
	}
}

func WithIncludeRoot(path string) LoadOption {
	return func(o *LoadOptions) { o.IncludeRoot = path }
}

func WithLimits(limits Limits) LoadOption {
	return func(o *LoadOptions) { o.Limits = limits }
}

func ParseString(source string) (Value, error) {
	doc, err := parseDocument(source, "")
	if err != nil {
		return nil, err
	}
	return newResolver(LoadOptions{Limits: defaultLimits()}).eval(doc)
}

func ParseFile(path string, opts ...LoadOption) (Value, error) {
	options := loadOptions(opts...)
	abs, _ := filepath.Abs(path)
	if options.IncludeRoot == "" {
		options.IncludeRoot = filepath.Dir(abs)
	}
	source, err := os.ReadFile(abs)
	if err != nil {
		return nil, simpleError(IncludeNotFound, "include file not found: "+err.Error())
	}
	if len(source) > options.Limits.MaxFileSize {
		return nil, simpleError(ResourceLimitExceeded, "maximum file size exceeded")
	}
	doc, err := parseDocument(string(source), filepath.Clean(abs))
	if err != nil {
		return nil, err
	}
	resolver := newResolver(options)
	resolver.stack = append(resolver.stack, filepath.Clean(abs))
	resolver.seen[filepath.Clean(abs)] = true
	return resolver.eval(doc)
}

func loadOptions(opts ...LoadOption) LoadOptions {
	options := LoadOptions{Limits: defaultLimits()}
	for _, opt := range opts {
		opt(&options)
	}
	if options.Limits.MaxFileSize == 0 {
		options.Limits = defaultLimits()
	}
	if options.IncludeRoot != "" {
		abs, _ := filepath.Abs(options.IncludeRoot)
		options.IncludeRoot = filepath.Clean(abs)
	}
	return options
}

type layer int
type kind int

const (
	layerBase layer = iota
	layerLocal
)
const (
	kindStructuralObject kind = iota
	kindOrdinaryValue
)

type evalEntry struct {
	value Value
	layer layer
	kind  kind
}

type evalObject struct {
	entries map[string]*evalEntry
	order   []string
}

func (*evalObject) isValue() {}

type resolver struct {
	options LoadOptions
	root    *evalObject
	stack   []string
	seen    map[string]bool
	cache   map[string]*document
	inProg  [][]string
}

func newResolver(options LoadOptions) *resolver {
	return &resolver{
		options: options,
		root:    newEvalObject(),
		seen:    map[string]bool{},
		cache:   map[string]*document{},
		inProg:  [][]string{{}},
	}
}

func (r *resolver) eval(doc *document) (Value, error) {
	if err := r.evalObject(doc.root, nil, doc.file); err != nil {
		return nil, err
	}
	return publicObject(r.root), nil
}

func (r *resolver) evalObject(obj *astObject, path []string, sourcePath string) error {
	if len(path) > r.options.Limits.MaxObjectDepth {
		return sconError(ResourceLimitExceeded, "maximum object depth exceeded", obj.span)
	}
	localSeen := false
	for _, member := range obj.members {
		switch m := member.(type) {
		case astObjectSpread:
			if localSeen {
				return sconError(InvalidSpread, "object spread must appear before local members", m.span)
			}
			target, err := r.lookup(m.sub.path, m.span)
			if err != nil {
				return err
			}
			spread, ok := target.value.(*evalObject)
			if !ok {
				return sconError(TypeMismatch, "object spread target is not an object", m.span)
			}
			obj, err := r.objectAt(path, m.span)
			if err != nil {
				return err
			}
			overlayBase(obj, spread)
		case astInclude:
			included, err := r.loadInclude(sourcePath, m)
			if err != nil {
				return err
			}
			if err := r.evalObject(included.root, path, included.file); err != nil {
				return err
			}
		case astField:
			localSeen = true
			if err := r.evalField(m, path, sourcePath); err != nil {
				return err
			}
		}
	}
	return nil
}

func (r *resolver) evalField(field astField, current []string, sourcePath string) error {
	target := append(append([]string{}, current...), pathNames(field.path)...)
	if obj, ok := field.value.(astObjectValue); ok {
		if err := r.ensureLocalObject(target, field.span); err != nil {
			return err
		}
		r.inProg = append(r.inProg, target)
		err := r.evalObject(obj.object, target, sourcePath)
		r.inProg = r.inProg[:len(r.inProg)-1]
		return err
	}
	value, err := r.evalValue(field.value, sourcePath)
	if err != nil {
		return err
	}
	return r.insertLocalValue(target, value, kindOrdinaryValue, field.span)
}

func (r *resolver) evalValue(value astValue, sourcePath string) (Value, error) {
	switch v := value.(type) {
	case astNull:
		return Null{}, nil
	case astBool:
		return Bool(v.value), nil
	case astNumber:
		n, err := ParseNumber(v.raw)
		if err != nil {
			return nil, sconError(InvalidNumber, err.Error(), v.span)
		}
		return n, nil
	case astString:
		return r.evalString(v)
	case astSubstitution:
		entry, err := r.lookup(v.path, v.span)
		if err != nil {
			return nil, err
		}
		return deepCopy(entry.value), nil
	case astArray:
		out := Array{}
		for _, item := range v.items {
			if len(out) >= r.options.Limits.MaxArrayLength {
				return nil, sconError(ResourceLimitExceeded, "maximum array length exceeded", item.itemSpan())
			}
			switch it := item.(type) {
			case astArrayValue:
				value, err := r.evalValue(it.value, sourcePath)
				if err != nil {
					return nil, err
				}
				out = append(out, value)
			case astArraySpread:
				entry, err := r.lookup(it.sub.path, it.span)
				if err != nil {
					return nil, err
				}
				values, ok := entry.value.(Array)
				if !ok {
					return nil, sconError(TypeMismatch, "array spread target is not an array", it.span)
				}
				for _, item := range values {
					out = append(out, deepCopy(item))
				}
			}
		}
		return out, nil
	case astObjectValue:
		nested := newResolver(r.options)
		nested.stack = r.stack
		nested.seen = r.seen
		nested.cache = r.cache
		if err := nested.evalObject(v.object, nil, sourcePath); err != nil {
			return nil, err
		}
		return publicObject(nested.root), nil
	}
	return nil, simpleError(UnexpectedToken, "unknown value")
}

func (r *resolver) evalString(value astString) (Value, error) {
	if len(value.parts) == 1 {
		if literal, ok := value.parts[0].(stringLiteralPart); ok {
			return String(literal.value), nil
		}
	}
	var out strings.Builder
	for _, part := range value.parts {
		switch p := part.(type) {
		case stringLiteralPart:
			out.WriteString(p.value)
		case stringInterpolationPart:
			entry, err := r.lookup(p.path, p.span)
			if err != nil {
				return nil, err
			}
			switch v := entry.value.(type) {
			case String:
				out.WriteString(string(v))
			case Number:
				out.WriteString(v.String())
			case Bool:
				if bool(v) {
					out.WriteString("true")
				} else {
					out.WriteString("false")
				}
			default:
				return nil, sconError(TypeMismatch, "interpolation requires string, number, or boolean", p.span)
			}
		}
	}
	return String(out.String()), nil
}

func (r *resolver) lookup(path astPath, span Span) (*evalEntry, error) {
	names := pathNames(path)
	for _, prog := range r.inProg {
		if equalPath(names, prog) {
			return nil, sconError(MissingReference, "reference is not completed yet", span)
		}
	}
	current := r.root
	var entry *evalEntry
	for i, name := range names {
		entry = current.entries[name]
		if entry == nil {
			return nil, sconError(MissingReference, "missing reference '"+name+"'", span)
		}
		if i < len(names)-1 {
			object, ok := entry.value.(*evalObject)
			if !ok {
				return nil, sconError(TypeMismatch, "reference path crosses non-object value", span)
			}
			current = object
		}
	}
	return entry, nil
}

func (r *resolver) ensureLocalObject(path []string, span Span) error {
	current := r.root
	for i, name := range path {
		entry := current.entries[name]
		if entry == nil {
			child := newEvalObject()
			current.entries[name] = &evalEntry{value: child, layer: layerLocal, kind: kindStructuralObject}
			current.order = append(current.order, name)
			current = child
			continue
		}
		object, ok := entry.value.(*evalObject)
		if !ok {
			return sconError(PathConflict, "path conflicts with scalar value", span)
		}
		if i == len(path)-1 {
			if entry.layer == layerLocal && entry.kind != kindStructuralObject {
				return sconError(PathConflict, "object field conflicts with ordinary value", span)
			}
			entry.layer = layerLocal
			entry.kind = kindStructuralObject
		}
		current = object
	}
	return nil
}

func (r *resolver) insertLocalValue(path []string, value Value, k kind, span Span) error {
	current := r.root
	for _, name := range path[:len(path)-1] {
		entry := current.entries[name]
		if entry == nil {
			child := newEvalObject()
			current.entries[name] = &evalEntry{value: child, layer: layerLocal, kind: kindStructuralObject}
			current.order = append(current.order, name)
			current = child
			continue
		}
		object, ok := entry.value.(*evalObject)
		if !ok {
			return sconError(PathConflict, "path conflicts with scalar value", span)
		}
		current = object
	}
	leaf := path[len(path)-1]
	existing := current.entries[leaf]
	if existing == nil {
		current.entries[leaf] = &evalEntry{value: value, layer: layerLocal, kind: k}
		current.order = append(current.order, leaf)
		return nil
	}
	if existing.layer == layerBase {
		overlayLocal(existing, value, k)
		return nil
	}
	if existing.kind == kindStructuralObject && k == kindStructuralObject {
		a, aok := existing.value.(*evalObject)
		b, bok := value.(*evalObject)
		if aok && bok {
			return mergeLocalObjects(a, b, span)
		}
	}
	return sconError(DuplicateKey, "duplicate key '"+leaf+"'", span)
}

func (r *resolver) objectAt(path []string, span Span) (*evalObject, error) {
	current := r.root
	for _, name := range path {
		entry := current.entries[name]
		if entry == nil {
			return nil, sconError(PathConflict, "target object does not exist", span)
		}
		object, ok := entry.value.(*evalObject)
		if !ok {
			return nil, sconError(PathConflict, "target path is not an object", span)
		}
		current = object
	}
	return current, nil
}

func (r *resolver) loadInclude(includingFile string, include astInclude) (*document, error) {
	includePath := include.path.value
	if invalidIncludePath(includePath) {
		return nil, sconError(InvalidIncludePath, "invalid include path", include.span)
	}
	includeRoot := r.options.IncludeRoot
	if includeRoot == "" {
		if includingFile == "" {
			return nil, sconError(InvalidIncludePath, "includes require a file context", include.span)
		}
		includeRoot = filepath.Dir(includingFile)
	}
	base := includeRoot
	if includingFile != "" {
		base = filepath.Dir(includingFile)
	}
	candidate, _ := filepath.Abs(filepath.Join(base, includePath))
	candidate = filepath.Clean(candidate)
	rootAbs, _ := filepath.Abs(includeRoot)
	rootAbs = filepath.Clean(rootAbs)
	if !withinRoot(candidate, rootAbs) {
		return nil, sconError(IncludePathDenied, "include path escapes include root", include.span)
	}
	if containsString(r.stack, candidate) {
		return nil, sconError(IncludeCycle, "include cycle: "+candidate, include.span)
	}
	if len(r.stack) >= r.options.Limits.MaxIncludeDepth {
		return nil, sconError(ResourceLimitExceeded, "maximum include depth exceeded", include.span)
	}
	r.seen[candidate] = true
	if len(r.seen) > r.options.Limits.MaxIncludeFiles {
		return nil, sconError(ResourceLimitExceeded, "maximum include file count exceeded", include.span)
	}
	if doc := r.cache[candidate]; doc != nil {
		return doc, nil
	}
	info, err := os.Stat(candidate)
	if err != nil {
		return nil, sconError(IncludeNotFound, "include file not found: "+err.Error(), include.span)
	}
	if info.IsDir() {
		return nil, sconError(IncludeNotFile, "include path is not a file", include.span)
	}
	if info.Size() > int64(r.options.Limits.MaxFileSize) {
		return nil, sconError(ResourceLimitExceeded, "maximum file size exceeded", include.span)
	}
	source, err := os.ReadFile(candidate)
	if err != nil {
		return nil, sconError(IncludeNotFound, "include file not found: "+err.Error(), include.span)
	}
	r.stack = append(r.stack, candidate)
	doc, err := parseDocument(string(source), candidate)
	r.stack = r.stack[:len(r.stack)-1]
	if err != nil {
		if se, ok := err.(*Error); ok {
			code := IncludeParseError
			if se.Code == InvalidRootType {
				code = IncludeRootTypeError
			}
			return nil, &Error{Code: code, Msg: se.Msg, Span: se.Span}
		}
		return nil, err
	}
	r.cache[candidate] = doc
	return doc, nil
}

func newEvalObject() *evalObject {
	return &evalObject{entries: map[string]*evalEntry{}}
}

func publicObject(object *evalObject) *Object {
	out := NewObject()
	for _, key := range object.order {
		if entry := object.entries[key]; entry != nil {
			out.Set(key, publicValue(entry.value))
		}
	}
	return out
}

func publicValue(v Value) Value {
	switch value := v.(type) {
	case *evalObject:
		return publicObject(value)
	case Array:
		out := make(Array, len(value))
		for i, item := range value {
			out[i] = publicValue(item)
		}
		return out
	default:
		return value
	}
}

func pathNames(path astPath) []string {
	out := make([]string, len(path.segments))
	for i, segment := range path.segments {
		out[i] = segment.value
	}
	return out
}

func equalPath(a, b []string) bool {
	if len(a) != len(b) {
		return false
	}
	for i := range a {
		if a[i] != b[i] {
			return false
		}
	}
	return true
}

func overlayBase(target *evalObject, source *evalObject) {
	for _, key := range source.order {
		entry := source.entries[key]
		existing := target.entries[key]
		if existing == nil {
			target.entries[key] = &evalEntry{value: deepCopy(entry.value), layer: layerBase, kind: kindOrdinaryValue}
			target.order = append(target.order, key)
		} else if existing.layer == layerBase {
			if a, ok := existing.value.(*evalObject); ok {
				if b, ok := entry.value.(*evalObject); ok {
					mergeOverride(a, b)
					continue
				}
			}
			existing.value = deepCopy(entry.value)
		}
	}
}

func overlayLocal(existing *evalEntry, value Value, k kind) {
	if a, ok := existing.value.(*evalObject); ok {
		if b, ok := value.(*evalObject); ok {
			mergeOverride(a, b)
			existing.layer = layerLocal
			existing.kind = k
			return
		}
	}
	existing.value = value
	existing.layer = layerLocal
	existing.kind = k
}

func mergeOverride(target, source *evalObject) {
	for _, key := range source.order {
		entry := source.entries[key]
		if existing := target.entries[key]; existing != nil {
			if a, ok := existing.value.(*evalObject); ok {
				if b, ok := entry.value.(*evalObject); ok {
					mergeOverride(a, b)
					continue
				}
			}
			existing.value = deepCopy(entry.value)
			continue
		}
		target.entries[key] = &evalEntry{value: deepCopy(entry.value), layer: entry.layer, kind: entry.kind}
		target.order = append(target.order, key)
	}
}

func mergeLocalObjects(target, source *evalObject, span Span) error {
	for _, key := range source.order {
		entry := source.entries[key]
		if target.entries[key] != nil {
			return sconError(DuplicateKey, "duplicate key '"+key+"'", span)
		}
		target.entries[key] = &evalEntry{value: deepCopy(entry.value), layer: entry.layer, kind: entry.kind}
		target.order = append(target.order, key)
	}
	return nil
}

func invalidIncludePath(path string) bool {
	return strings.Contains(path, "://") ||
		strings.HasPrefix(path, "classpath:") ||
		strings.Contains(path, "*") ||
		strings.HasPrefix(path, "~") ||
		strings.HasPrefix(path, "$") ||
		strings.HasPrefix(path, "/") ||
		looksLikeWindowsAbsolute(path)
}

func looksLikeWindowsAbsolute(path string) bool {
	return len(path) >= 3 && path[1] == ':' && (path[2] == '/' || path[2] == '\\')
}

func withinRoot(path, root string) bool {
	if runtime.GOOS == "windows" {
		path = strings.ToLower(path)
		root = strings.ToLower(root)
	}
	rel, err := filepath.Rel(root, path)
	return err == nil && rel != ".." && !strings.HasPrefix(rel, ".."+string(filepath.Separator))
}

func containsString(values []string, needle string) bool {
	for _, value := range values {
		if value == needle {
			return true
		}
	}
	return false
}
