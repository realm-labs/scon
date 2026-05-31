package scon

import (
	"reflect"
)

func Unmarshal(source []byte, out any) error {
	value, err := ParseString(string(source))
	if err != nil {
		return err
	}
	return decodeReflect(value, reflect.ValueOf(out))
}

func UnmarshalFile(path string, out any, opts ...LoadOption) error {
	value, err := ParseFile(path, opts...)
	if err != nil {
		return err
	}
	return decodeReflect(value, reflect.ValueOf(out))
}

func decodeReflect(value Value, target reflect.Value) error {
	if target.Kind() != reflect.Pointer || target.IsNil() {
		return simpleError(Serde, "decode target must be a non-nil pointer")
	}
	return assignValue(value, target.Elem())
}

func assignValue(value Value, target reflect.Value) error {
	if !target.CanSet() {
		return nil
	}
	if target.Kind() == reflect.Pointer {
		if _, ok := value.(Null); ok {
			target.Set(reflect.Zero(target.Type()))
			return nil
		}
		target.Set(reflect.New(target.Type().Elem()))
		return assignValue(value, target.Elem())
	}
	switch target.Kind() {
	case reflect.Bool:
		v, ok := value.(Bool)
		if !ok {
			return simpleError(Serde, "expected bool")
		}
		target.SetBool(bool(v))
	case reflect.String:
		v, ok := value.(String)
		if !ok {
			return simpleError(Serde, "expected string")
		}
		target.SetString(string(v))
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		n, ok := value.(Number)
		if !ok {
			return simpleError(Serde, "expected number")
		}
		i, ok := n.Int64()
		if !ok || target.OverflowInt(i) {
			return simpleError(Serde, "integer overflow")
		}
		target.SetInt(i)
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		n, ok := value.(Number)
		if !ok {
			return simpleError(Serde, "expected number")
		}
		u, ok := n.Uint64()
		if !ok || target.OverflowUint(u) {
			return simpleError(Serde, "integer overflow")
		}
		target.SetUint(u)
	case reflect.Float32, reflect.Float64:
		n, ok := value.(Number)
		if !ok {
			return simpleError(Serde, "expected number")
		}
		f := n.Float64()
		if target.OverflowFloat(f) {
			return simpleError(Serde, "float overflow")
		}
		target.SetFloat(f)
	case reflect.Slice:
		values, ok := value.(Array)
		if !ok {
			return simpleError(Serde, "expected array")
		}
		slice := reflect.MakeSlice(target.Type(), len(values), len(values))
		for i, item := range values {
			if err := assignValue(item, slice.Index(i)); err != nil {
				return err
			}
		}
		target.Set(slice)
	case reflect.Array:
		values, ok := value.(Array)
		if !ok {
			return simpleError(Serde, "expected array")
		}
		if len(values) != target.Len() {
			return simpleError(Serde, "array length mismatch")
		}
		for i, item := range values {
			if err := assignValue(item, target.Index(i)); err != nil {
				return err
			}
		}
	case reflect.Map:
		object, ok := value.(*Object)
		if !ok {
			return simpleError(Serde, "expected object")
		}
		if target.Type().Key().Kind() != reflect.String {
			return simpleError(Serde, "SCON map keys must be strings")
		}
		m := reflect.MakeMap(target.Type())
		for _, entry := range object.entries {
			item := reflect.New(target.Type().Elem()).Elem()
			if err := assignValue(entry.Value, item); err != nil {
				return err
			}
			m.SetMapIndex(reflect.ValueOf(entry.Key).Convert(target.Type().Key()), item)
		}
		target.Set(m)
	case reflect.Struct:
		object, ok := value.(*Object)
		if !ok {
			return simpleError(Serde, "expected object")
		}
		t := target.Type()
		for i := range t.NumField() {
			field := t.Field(i)
			if field.PkgPath != "" {
				continue
			}
			name := fieldName(field)
			if name == "-" {
				continue
			}
			if item, ok := object.Get(name); ok {
				if err := assignValue(item, target.Field(i)); err != nil {
					return err
				}
			}
		}
	default:
		return simpleError(Serde, "unsupported target kind: "+target.Kind().String())
	}
	return nil
}
