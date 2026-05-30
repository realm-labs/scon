package io.github.realmlabs.scon.idea

import com.intellij.lexer.LexerBase
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType

class SconIdeaLexer : LexerBase() {
    private var buffer: CharSequence = ""
    private var startOffset: Int = 0
    private var endOffset: Int = 0
    private var tokenStart: Int = 0
    private var tokenEnd: Int = 0
    private var tokenType: IElementType? = null

    override fun start(buffer: CharSequence, startOffset: Int, endOffset: Int, initialState: Int) {
        this.buffer = buffer
        this.startOffset = startOffset
        this.endOffset = endOffset
        this.tokenStart = startOffset
        locateToken()
    }

    override fun getState(): Int = 0
    override fun getTokenType(): IElementType? = tokenType
    override fun getTokenStart(): Int = tokenStart
    override fun getTokenEnd(): Int = tokenEnd
    override fun getBufferSequence(): CharSequence = buffer
    override fun getBufferEnd(): Int = endOffset

    override fun advance() {
        tokenStart = tokenEnd
        locateToken()
    }

    private fun locateToken() {
        if (tokenStart >= endOffset) {
            tokenType = null
            tokenEnd = endOffset
            return
        }
        val ch = buffer[tokenStart]
        when {
            ch == ' ' || ch == '\t' || ch == '\n' || ch == '\r' -> lexWhitespace()
            ch == '#' -> lexLineComment()
            ch == '/' && peek(1) == '/' -> lexLineComment()
            ch == '"' -> lexString()
            ch == '$' && peek(1) == '{' -> finish(SconTokenTypes.SUBSTITUTION_START, tokenStart + 2)
            ch == '{' -> finish(SconTokenTypes.LEFT_BRACE, tokenStart + 1)
            ch == '}' -> finish(SconTokenTypes.RIGHT_BRACE, tokenStart + 1)
            ch == '[' -> finish(SconTokenTypes.LEFT_BRACKET, tokenStart + 1)
            ch == ']' -> finish(SconTokenTypes.RIGHT_BRACKET, tokenStart + 1)
            ch == '=' -> finish(SconTokenTypes.EQUALS, tokenStart + 1)
            ch == ',' -> finish(SconTokenTypes.COMMA, tokenStart + 1)
            ch == '.' && peek(1) == '.' && peek(2) == '.' -> finish(SconTokenTypes.SPREAD, tokenStart + 3)
            ch == '.' -> finish(SconTokenTypes.DOT, tokenStart + 1)
            ch == '-' || ch.isDigit() -> lexNumber()
            ch.isIdentifierStart() -> lexIdentifier()
            else -> finish(TokenType.BAD_CHARACTER, tokenStart + 1)
        }
    }

    private fun lexWhitespace() {
        var i = tokenStart + 1
        while (i < endOffset && buffer[i].isWhitespace()) i++
        finish(TokenType.WHITE_SPACE, i)
    }

    private fun lexLineComment() {
        var i = tokenStart + 1
        if (buffer[tokenStart] == '/') i++
        while (i < endOffset && buffer[i] != '\n' && buffer[i] != '\r') i++
        finish(SconTokenTypes.COMMENT, i)
    }

    private fun lexString() {
        var i = tokenStart + 1
        while (i < endOffset) {
            when (buffer[i]) {
                '"' -> {
                    finish(SconTokenTypes.STRING, i + 1)
                    return
                }
                '\\' -> i += 2
                '\n', '\r' -> {
                    finish(SconTokenTypes.STRING, i)
                    return
                }
                else -> i++
            }
        }
        finish(SconTokenTypes.STRING, endOffset)
    }

    private fun lexNumber() {
        var i = tokenStart
        if (i < endOffset && buffer[i] == '-') i++
        while (i < endOffset && buffer[i].isDigit()) i++
        if (i < endOffset && buffer[i] == '.') {
            i++
            while (i < endOffset && buffer[i].isDigit()) i++
        }
        if (i < endOffset && (buffer[i] == 'e' || buffer[i] == 'E')) {
            i++
            if (i < endOffset && (buffer[i] == '+' || buffer[i] == '-')) i++
            while (i < endOffset && buffer[i].isDigit()) i++
        }
        finish(SconTokenTypes.NUMBER, i.coerceAtLeast(tokenStart + 1))
    }

    private fun lexIdentifier() {
        var i = tokenStart + 1
        while (i < endOffset && buffer[i].isIdentifierPart()) i++
        val text = buffer.subSequence(tokenStart, i).toString()
        val type = when (text) {
            "true", "false" -> SconTokenTypes.BOOLEAN
            "null" -> SconTokenTypes.NULL
            "include" -> SconTokenTypes.INCLUDE
            else -> SconTokenTypes.IDENTIFIER
        }
        finish(type, i)
    }

    private fun finish(type: IElementType, end: Int) {
        tokenType = type
        tokenEnd = end.coerceAtMost(endOffset)
    }

    private fun peek(offset: Int): Char? =
        buffer.getOrNull(tokenStart + offset)
}

private fun Char.isIdentifierStart(): Boolean =
    this in 'A'..'Z' || this in 'a'..'z' || this == '_'

private fun Char.isIdentifierPart(): Boolean =
    isIdentifierStart() || this in '0'..'9' || this == '-'
