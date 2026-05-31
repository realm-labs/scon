# Java SCON

Java 17 implementation of SCON core parsing, resolution, canonical value
formatting, reflection-based typed mapping, and a separate Jackson adapter.

```java
record Config(String name, int port) {}

Config config = SconMapper.readValue("name = \"demo\"\nport = 8080", Config.class);
```

Useful commands:

```sh
mvn test
```
