package io.github.realmlabs.scon.idea

import com.intellij.psi.TokenType
import kotlin.test.Test
import kotlin.test.assertEquals

class SconIdeaLexerTest {
    @Test
    fun tokenizesEditorSyntax() {
        val lexer = SconIdeaLexer()
        lexer.start("""name = "demo" # comment""")

        assertEquals(SconTokenTypes.IDENTIFIER, lexer.tokenType)
        lexer.advance()
        assertEquals(TokenType.WHITE_SPACE, lexer.tokenType)
        lexer.advance()
        assertEquals(SconTokenTypes.EQUALS, lexer.tokenType)
        lexer.advance()
        assertEquals(TokenType.WHITE_SPACE, lexer.tokenType)
        lexer.advance()
        assertEquals(SconTokenTypes.STRING, lexer.tokenType)
    }
}
