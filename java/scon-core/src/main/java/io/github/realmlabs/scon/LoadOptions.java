package io.github.realmlabs.scon;

import java.nio.file.Path;

public record LoadOptions(
    Path includeRoot,
    int maxFileSize,
    int maxIncludeDepth,
    int maxIncludeFiles,
    int maxArrayLength,
    int maxObjectDepth
) {
    public LoadOptions {
        if (maxFileSize == 0) maxFileSize = 16 * 1024 * 1024;
        if (maxIncludeDepth == 0) maxIncludeDepth = 64;
        if (maxIncludeFiles == 0) maxIncludeFiles = 1024;
        if (maxArrayLength == 0) maxArrayLength = 1_000_000;
        if (maxObjectDepth == 0) maxObjectDepth = 512;
    }

    public static LoadOptions defaults() {
        return new LoadOptions(null, 0, 0, 0, 0, 0);
    }

    public LoadOptions withIncludeRoot(Path includeRoot) {
        return new LoadOptions(includeRoot, maxFileSize, maxIncludeDepth, maxIncludeFiles, maxArrayLength, maxObjectDepth);
    }
}
