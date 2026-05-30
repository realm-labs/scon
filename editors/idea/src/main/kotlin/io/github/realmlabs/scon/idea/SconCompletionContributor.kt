package io.github.realmlabs.scon.idea

import com.intellij.codeInsight.completion.CompletionContributor
import com.intellij.codeInsight.completion.CompletionParameters
import com.intellij.codeInsight.completion.CompletionResultSet
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.openapi.vfs.VirtualFile

class SconCompletionContributor : CompletionContributor() {
    override fun fillCompletionVariants(parameters: CompletionParameters, result: CompletionResultSet) {
        val file = parameters.originalFile as? SconFile ?: return
        val text = file.text
        val offset = parameters.offset.coerceIn(0, text.length)

        val substitutionPrefix = pathPrefixAtOffsetInSubstitution(text, offset)
        if (substitutionPrefix != null) {
            val prefixMatcher = result.withPrefixMatcher(substitutionPrefix.substringAfterLast('.'))
            runCatching { file.parseSconDocument().collectDefinitionPaths() }
                .getOrDefault(emptyList())
                .forEach { definition ->
                    prefixMatcher.addElement(
                        LookupElementBuilder.create(definition.dotted)
                            .withTypeText("SCON path", true),
                    )
                }
            return
        }

        if (isIncludeStringPosition(text, offset)) {
            val base = file.virtualFile?.parent ?: return
            base.children
                .filter { !it.isDirectory && it.extension == "scon" }
                .sortedBy(VirtualFile::getName)
                .forEach {
                    result.addElement(
                        LookupElementBuilder.create(it.name)
                            .withTypeText("include", true),
                    )
                }
        }
    }

    private fun isIncludeStringPosition(text: String, offset: Int): Boolean {
        val lineStart = text.lastIndexOf('\n', (offset - 1).coerceAtLeast(0)).let { if (it < 0) 0 else it + 1 }
        val beforeCaret = text.substring(lineStart, offset)
        return beforeCaret.matches(Regex("\\s*include\\s+\"[^\"]*"))
    }
}
