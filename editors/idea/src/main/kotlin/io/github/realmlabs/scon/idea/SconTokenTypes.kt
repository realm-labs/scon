package io.github.realmlabs.scon.idea

import com.intellij.psi.tree.IElementType

class SconElementType(debugName: String) : IElementType(debugName, SconLanguage)

object SconTokenTypes {
    val IDENTIFIER = SconElementType("SCON_IDENTIFIER")
    val STRING = SconElementType("SCON_STRING")
    val NUMBER = SconElementType("SCON_NUMBER")
    val BOOLEAN = SconElementType("SCON_BOOLEAN")
    val NULL = SconElementType("SCON_NULL")
    val INCLUDE = SconElementType("SCON_INCLUDE")
    val COMMENT = SconElementType("SCON_COMMENT")
    val SUBSTITUTION_START = SconElementType("SCON_SUBSTITUTION_START")
    val SPREAD = SconElementType("SCON_SPREAD")
    val LEFT_BRACE = SconElementType("SCON_LEFT_BRACE")
    val RIGHT_BRACE = SconElementType("SCON_RIGHT_BRACE")
    val LEFT_BRACKET = SconElementType("SCON_LEFT_BRACKET")
    val RIGHT_BRACKET = SconElementType("SCON_RIGHT_BRACKET")
    val EQUALS = SconElementType("SCON_EQUALS")
    val DOT = SconElementType("SCON_DOT")
    val COMMA = SconElementType("SCON_COMMA")
}
