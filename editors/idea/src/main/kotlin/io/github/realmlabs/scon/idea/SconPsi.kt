package io.github.realmlabs.scon.idea

import com.intellij.extapi.psi.PsiFileBase
import com.intellij.lang.ASTNode
import com.intellij.lang.ParserDefinition
import com.intellij.lang.PsiParser
import com.intellij.lang.SyntaxTreeBuilder
import com.intellij.lexer.Lexer
import com.intellij.openapi.project.Project
import com.intellij.psi.FileViewProvider
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.TokenType
import com.intellij.psi.tree.IFileElementType
import com.intellij.psi.tree.TokenSet
import com.intellij.extapi.psi.ASTWrapperPsiElement

class SconFile(viewProvider: FileViewProvider) : PsiFileBase(viewProvider, SconLanguage) {
    override fun getFileType(): SconFileType = SconFileType.INSTANCE
    override fun toString(): String = "SCON File"
}

object SconElementTypes {
    val FILE = IFileElementType(SconLanguage)
    val ROOT = SconElementType("SCON_ROOT")
}

class SconParserDefinition : ParserDefinition {
    override fun createLexer(project: Project?): Lexer = SconIdeaLexer()

    override fun createParser(project: Project?): PsiParser =
        PsiParser { root, builder ->
            val marker = builder.mark()
            while (!builder.eof()) {
                builder.advanceLexer()
            }
            marker.done(root)
            builder.treeBuilt
        }

    override fun getFileNodeType(): IFileElementType = SconElementTypes.FILE
    override fun getWhitespaceTokens(): TokenSet = TokenSet.create(TokenType.WHITE_SPACE)
    override fun getCommentTokens(): TokenSet = TokenSet.create(SconTokenTypes.COMMENT)
    override fun getStringLiteralElements(): TokenSet = TokenSet.create(SconTokenTypes.STRING)
    override fun createElement(node: ASTNode): PsiElement = ASTWrapperPsiElement(node)
    override fun createFile(viewProvider: FileViewProvider): PsiFile = SconFile(viewProvider)
}
