package io.github.realmlabs.scon.idea

import com.intellij.codeInsight.navigation.actions.GotoDeclarationHandler
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiManager
import java.nio.file.Path

class SconGotoDeclarationHandler : GotoDeclarationHandler {
    override fun getGotoDeclarationTargets(
        sourceElement: PsiElement?,
        offset: Int,
        editor: Editor,
    ): Array<PsiElement>? {
        val file = sourceElement?.containingFile as? SconFile ?: return null
        val text = file.text

        val includePath = includePathAtOffset(text, offset)
        if (includePath != null) {
            val base = file.virtualFile?.parent ?: return null
            val target = base.findFileByRelativePath(includePath) ?: return null
            return PsiManager.getInstance(file.project).findFile(target)?.let { arrayOf(it) }
        }

        val path = pathAtOffsetInSubstitution(text, offset) ?: return null
        val definition = runCatching { file.parseSconDocument().findDefinition(path) }.getOrNull() ?: return null
        return file.findElementAt(definition.span.start)?.let { arrayOf(it) }
    }

    private fun includePathAtOffset(text: String, offset: Int): String? {
        val lineStart = text.lastIndexOf('\n', (offset - 1).coerceAtLeast(0)).let { if (it < 0) 0 else it + 1 }
        val lineEnd = text.indexOf('\n', offset).let { if (it < 0) text.length else it }
        val line = text.substring(lineStart, lineEnd)
        val match = Regex("""^\s*include\s+"([^"]+)"""").find(line) ?: return null
        val start = lineStart + match.range.first + match.value.indexOf('"') + 1
        val end = start + match.groupValues[1].length
        if (offset !in start..end) return null
        return match.groupValues[1].takeIf { !Path.of(it).isAbsolute }
    }
}
