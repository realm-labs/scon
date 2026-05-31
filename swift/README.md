# Swift SCON

Swift Package implementation of SCON core parsing, resolution, canonical value
formatting, and Codable-based typed encoding/decoding.

```swift
struct Config: Codable, Equatable {
    let name: String
    let port: Int
}

let config = try Scon.decode("name = \"demo\"\nport = 8080", as: Config.self)
```

Useful commands:

```sh
swift test
```
