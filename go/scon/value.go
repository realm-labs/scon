package scon

type Value interface {
	isValue()
}

type Null struct{}
type Bool bool
type String string
type Array []Value

type Object struct {
	entries []Entry
	index   map[string]int
}

type Entry struct {
	Key   string
	Value Value
}

func (Null) isValue()    {}
func (Bool) isValue()    {}
func (Number) isValue()  {}
func (String) isValue()  {}
func (Array) isValue()   {}
func (*Object) isValue() {}

func NewObject() *Object {
	return &Object{index: map[string]int{}}
}

func (o *Object) Set(key string, value Value) {
	if o.index == nil {
		o.index = map[string]int{}
	}
	if idx, ok := o.index[key]; ok {
		o.entries[idx].Value = value
		return
	}
	o.index[key] = len(o.entries)
	o.entries = append(o.entries, Entry{Key: key, Value: value})
}

func (o *Object) Get(key string) (Value, bool) {
	if o == nil || o.index == nil {
		return nil, false
	}
	idx, ok := o.index[key]
	if !ok {
		return nil, false
	}
	return o.entries[idx].Value, true
}

func (o *Object) Entries() []Entry {
	if o == nil {
		return nil
	}
	out := make([]Entry, len(o.entries))
	copy(out, o.entries)
	return out
}

func (o *Object) Len() int {
	if o == nil {
		return 0
	}
	return len(o.entries)
}

func cloneObject(o *Object) *Object {
	out := NewObject()
	for _, entry := range o.entries {
		out.Set(entry.Key, deepCopy(entry.Value))
	}
	return out
}

func deepCopy(v Value) Value {
	switch value := v.(type) {
	case Null:
		return value
	case Bool:
		return value
	case Number:
		return value
	case String:
		return value
	case Array:
		out := make(Array, len(value))
		for i, item := range value {
			out[i] = deepCopy(item)
		}
		return out
	case *Object:
		return cloneObject(value)
	case *evalObject:
		out := newEvalObject()
		for _, key := range value.order {
			entry := value.entries[key]
			out.entries[key] = &evalEntry{
				value: deepCopy(entry.value),
				layer: entry.layer,
				kind:  entry.kind,
			}
			out.order = append(out.order, key)
		}
		return out
	default:
		return value
	}
}
