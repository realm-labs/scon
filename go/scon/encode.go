package scon

import (
	"math"
	"reflect"
	"strings"
)

func Marshal(v any) ([]byte, error) {
	value, err := encodeReflect(reflect.ValueOf(v))
	if err != nil {
		return nil, err
	}
	text, err := FormatValue(value)
	if err != nil {
		return nil, err
	}
	return []byte(text), nil
}

func encodeReflect(value reflect.Value) (Value, error) {
	if !value.IsValid() {
		return Null{}, nil
	}
	if value.Kind() == reflect.Pointer || value.Kind() == reflect.Interface {
		if value.IsNil() {
			return Null{}, nil
		}
		return encodeReflect(value.Elem())
	}
	switch value.Kind() {
	case reflect.Bool:
		return Bool(value.Bool()), nil
	case reflect.String:
		return String(value.String()), nil
	case reflect.Int, reflect.Int8, reflect.Int16, reflect.Int32, reflect.Int64:
		return numberFromReflectSigned(value.Int()), nil
	case reflect.Uint, reflect.Uint8, reflect.Uint16, reflect.Uint32, reflect.Uint64, reflect.Uintptr:
		return numberFromReflectUnsigned(value.Uint()), nil
	case reflect.Float32, reflect.Float64:
		f := value.Float()
		if math.IsNaN(f) || math.IsInf(f, 0) {
			return nil, simpleError(Serde, "non-finite floats cannot be serialized")
		}
		return numberFromReflectFloat(f)
	case reflect.Slice, reflect.Array:
		out := make(Array, value.Len())
		for i := range value.Len() {
			item, err := encodeReflect(value.Index(i))
			if err != nil {
				return nil, err
			}
			out[i] = item
		}
		return out, nil
	case reflect.Map:
		if value.Type().Key().Kind() != reflect.String {
			return nil, simpleError(Serde, "SCON map keys must be strings")
		}
		out := NewObject()
		iter := value.MapRange()
		for iter.Next() {
			item, err := encodeReflect(iter.Value())
			if err != nil {
				return nil, err
			}
			out.Set(iter.Key().String(), item)
		}
		return out, nil
	case reflect.Struct:
		out := NewObject()
		t := value.Type()
		for i := range t.NumField() {
			field := t.Field(i)
			if field.PkgPath != "" {
				continue
			}
			name := fieldName(field)
			if name == "-" {
				continue
			}
			item, err := encodeReflect(value.Field(i))
			if err != nil {
				return nil, err
			}
			out.Set(name, item)
		}
		return out, nil
	default:
		return nil, simpleError(Serde, "unsupported value kind: "+value.Kind().String())
	}
}

func fieldName(field reflect.StructField) string {
	tag := field.Tag.Get("scon")
	if tag != "" {
		return strings.Split(tag, ",")[0]
	}
	return field.Name
}
