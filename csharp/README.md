# C# SCON

C# implementation of SCON core parsing, resolution, canonical value
formatting, reflection-based typed mapping, and a System.Text.Json adapter.

```csharp
public record Config(string Name, int Port);

var config = SconMapper.Deserialize<Config>("Name = \"demo\"\nPort = 8080");
```

Useful commands:

```sh
dotnet test
```
