# Go SCON

Go implementation of SCON core parsing, resolution, canonical value formatting,
and reflection-based typed encoding/decoding.

```go
var cfg struct {
    Name string `scon:"name"`
    Port int    `scon:"port"`
}

err := scon.Unmarshal([]byte(`name = "demo"\nport = 8080`), &cfg)
```

Useful commands:

```sh
go test ./...
go vet ./...
```
