# SCON IntelliJ Plugin

This is the native JetBrains Platform frontend for SCON. It intentionally does
not use `scon-lsp` or LSP4IJ. Shared SCON language behavior belongs in
`../../kotlin/scon-core`; this plugin owns IntelliJ file type registration,
syntax highlighting, diagnostics, completion, navigation, documentation,
structure view, editor integration, and packaging.

Useful commands from the Kotlin build root:

```sh
cd kotlin
./gradlew :idea-plugin:runIde
./gradlew :idea-plugin:buildPlugin
./gradlew :idea-plugin:verifyPlugin
```

Marketplace publishing uses environment variables for signing and upload
credentials; do not commit certificates, private keys, or tokens.

Implemented native features:

- `.scon` file type registration
- syntax highlighting
- line comments and brace matching
- PSI file/parser shell for IntelliJ integration
- diagnostics from `kotlin/scon-core`
- path completion inside `${...}`
- local include path completion
- go to definition for substitutions and includes
- hover documentation for resolved substitution values
- structure view for object paths
- `Code > Format SCON File`, backed by `kotlin/scon-core.formatSource`
