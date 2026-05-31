package io.github.realmlabs.scon;

public class SconException extends RuntimeException {
    private final ErrorCode code;
    private final Span span;

    public SconException(ErrorCode code, String message) {
        this(code, message, null);
    }

    public SconException(ErrorCode code, String message, Span span) {
        super(code + ": " + message);
        this.code = code;
        this.span = span;
    }

    public ErrorCode code() {
        return code;
    }

    public Span span() {
        return span;
    }
}
