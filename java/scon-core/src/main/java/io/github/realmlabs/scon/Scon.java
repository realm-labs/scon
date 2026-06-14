package io.github.realmlabs.scon;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;

public final class Scon {
    private Scon() {}

    public static Analysis.ParsedSource parseSource(String source) {
        return Analysis.parseSource(source, null);
    }

    public static Analysis.DocumentAnalysis analyzeSource(String source) {
        return Analysis.analyzeSource(source, null);
    }

    public static SconValue parseString(String source) {
        return new Resolver(LoadOptions.defaults()).eval(Parser.parseDocument(source, null));
    }

    public static SconValue parseFile(Path path) {
        return parseFile(path, LoadOptions.defaults());
    }

    public static SconValue parseFile(Path path, LoadOptions options) {
        try {
            Path file = path.toAbsolutePath().normalize();
            Path root = options.includeRoot() == null ? file.getParent() : options.includeRoot().toAbsolutePath().normalize();
            String source = Files.readString(file);
            if (source.getBytes(java.nio.charset.StandardCharsets.UTF_8).length > options.maxFileSize()) {
                throw new SconException(ErrorCode.ResourceLimitExceeded, "maximum file size exceeded");
            }
            Resolver resolver = new Resolver(options.withIncludeRoot(root));
            resolver.stack.add(file);
            resolver.seen.add(file);
            return resolver.eval(Parser.parseDocument(source, file.toString()));
        } catch (IOException ex) {
            throw new SconException(ErrorCode.IncludeNotFound, "include file not found: " + ex.getMessage());
        }
    }

    public static String formatValue(SconValue value) {
        return Formatter.formatValue(value);
    }

    public static String formatSource(String source) {
        return SourceFormatter.formatSource(source);
    }

    public static SconValue getPath(SconValue value, String path) {
        SconValue current = value;
        for (String segment : path.split("\\.")) {
            if (!(current instanceof SconObject object)) {
                throw new SconException(ErrorCode.TypeMismatch, "path segment requires object");
            }
            current = object.get(segment);
            if (current == null) {
                throw new SconException(ErrorCode.MissingReference, "path is not defined");
            }
        }
        return current;
    }
}
