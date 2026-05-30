# SCON IntelliJ Plugin Release Checklist

The plugin is Marketplace-ready but publishing is intentionally manual until
release signing secrets are configured.

## Local Verification

```sh
cd kotlin
./gradlew :scon-core:test
./gradlew :idea-plugin:test
./gradlew :idea-plugin:buildPlugin
./gradlew :idea-plugin:verifyPlugin
```

Install the generated zip from `editors/idea/build/distributions/` into a clean
IntelliJ IDEA Community sandbox and verify:

- `.scon` files open with SCON file type.
- Syntax highlighting, comments, and braces work.
- Invalid files report diagnostics.
- `${...}` completion and go-to-definition work.
- `include "..."` path completion and navigation work.
- Structure view lists object paths.
- `Code > Format SCON File` runs without corrupting source.

## Marketplace Publishing

Set these environment variables outside the repository:

- `JETBRAINS_MARKETPLACE_TOKEN`
- `JETBRAINS_CERTIFICATE_CHAIN`
- `JETBRAINS_PRIVATE_KEY`
- `JETBRAINS_PRIVATE_KEY_PASSWORD`

Then run:

```sh
cd kotlin
./gradlew :idea-plugin:signPlugin
./gradlew :idea-plugin:publishPlugin
```

Do not commit certificates, private keys, tokens, generated plugin zips, or
build reports.
