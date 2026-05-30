package io.github.realmlabs.scon.idea

import com.intellij.lang.documentation.AbstractDocumentationProvider
import com.intellij.psi.PsiElement

class SconDocumentationProvider : AbstractDocumentationProvider() {
    override fun generateDoc(element: PsiElement, originalElement: PsiElement?): String? {
        val file = originalElement?.containingFile as? SconFile ?: element.containingFile as? SconFile ?: return null
        val offset = originalElement?.textOffset ?: element.textOffset
        val path = pathAtOffsetInSubstitution(file.text, offset) ?: return null
        val value = runCatching { file.resolveSconDocument().getPath(path) }.getOrNull() ?: return null
        return """
            <b>$path</b><br/>
            Type: <code>${value.typeName()}</code><br/>
            Value: <code>${value.preview()}</code>
        """.trimIndent()
    }
}
