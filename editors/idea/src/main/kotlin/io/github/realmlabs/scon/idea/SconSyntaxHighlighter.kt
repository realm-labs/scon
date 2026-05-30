package io.github.realmlabs.scon.idea

import com.intellij.lexer.Lexer
import com.intellij.openapi.editor.DefaultLanguageHighlighterColors
import com.intellij.openapi.editor.HighlighterColors
import com.intellij.openapi.editor.colors.TextAttributesKey
import com.intellij.openapi.fileTypes.SyntaxHighlighter
import com.intellij.openapi.fileTypes.SyntaxHighlighterFactory
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IElementType

class SconSyntaxHighlighter : SyntaxHighlighter {
    override fun getHighlightingLexer(): Lexer = SconIdeaLexer()

    override fun getTokenHighlights(tokenType: IElementType): Array<TextAttributesKey> =
        when (tokenType) {
            SconTokenTypes.COMMENT -> COMMENT_KEYS
            SconTokenTypes.STRING -> STRING_KEYS
            SconTokenTypes.NUMBER -> NUMBER_KEYS
            SconTokenTypes.BOOLEAN, SconTokenTypes.NULL -> KEYWORD_KEYS
            SconTokenTypes.INCLUDE -> KEYWORD_KEYS
            SconTokenTypes.IDENTIFIER -> IDENTIFIER_KEYS
            SconTokenTypes.SUBSTITUTION_START -> SUBSTITUTION_KEYS
            SconTokenTypes.SPREAD, SconTokenTypes.EQUALS, SconTokenTypes.DOT -> OPERATOR_KEYS
            SconTokenTypes.LEFT_BRACE,
            SconTokenTypes.RIGHT_BRACE,
            SconTokenTypes.LEFT_BRACKET,
            SconTokenTypes.RIGHT_BRACKET,
            SconTokenTypes.COMMA -> BRACES_KEYS
            TokenType.BAD_CHARACTER -> BAD_KEYS
            else -> TextAttributesKey.EMPTY_ARRAY
        }

    companion object {
        val COMMENT = TextAttributesKey.createTextAttributesKey("SCON_COMMENT", DefaultLanguageHighlighterColors.LINE_COMMENT)
        val STRING = TextAttributesKey.createTextAttributesKey("SCON_STRING", DefaultLanguageHighlighterColors.STRING)
        val NUMBER = TextAttributesKey.createTextAttributesKey("SCON_NUMBER", DefaultLanguageHighlighterColors.NUMBER)
        val KEYWORD = TextAttributesKey.createTextAttributesKey("SCON_KEYWORD", DefaultLanguageHighlighterColors.KEYWORD)
        val IDENTIFIER = TextAttributesKey.createTextAttributesKey("SCON_IDENTIFIER", DefaultLanguageHighlighterColors.INSTANCE_FIELD)
        val SUBSTITUTION = TextAttributesKey.createTextAttributesKey("SCON_SUBSTITUTION", DefaultLanguageHighlighterColors.METADATA)
        val OPERATOR = TextAttributesKey.createTextAttributesKey("SCON_OPERATOR", DefaultLanguageHighlighterColors.OPERATION_SIGN)
        val BRACES = TextAttributesKey.createTextAttributesKey("SCON_BRACES", DefaultLanguageHighlighterColors.BRACES)
        val BAD = TextAttributesKey.createTextAttributesKey("SCON_BAD_CHARACTER", HighlighterColors.BAD_CHARACTER)

        private val COMMENT_KEYS = arrayOf(COMMENT)
        private val STRING_KEYS = arrayOf(STRING)
        private val NUMBER_KEYS = arrayOf(NUMBER)
        private val KEYWORD_KEYS = arrayOf(KEYWORD)
        private val IDENTIFIER_KEYS = arrayOf(IDENTIFIER)
        private val SUBSTITUTION_KEYS = arrayOf(SUBSTITUTION)
        private val OPERATOR_KEYS = arrayOf(OPERATOR)
        private val BRACES_KEYS = arrayOf(BRACES)
        private val BAD_KEYS = arrayOf(BAD)
    }
}

class SconSyntaxHighlighterFactory : SyntaxHighlighterFactory() {
    override fun getSyntaxHighlighter(project: Project?, virtualFile: VirtualFile?): SyntaxHighlighter =
        SconSyntaxHighlighter()
}
