package io.github.realmlabs.scon.idea

import com.intellij.lang.annotation.AnnotationHolder
import com.intellij.lang.annotation.Annotator
import com.intellij.lang.annotation.HighlightSeverity
import com.intellij.psi.PsiElement
import io.github.realmlabs.scon.SconException

class SconAnnotator : Annotator {
    override fun annotate(element: PsiElement, holder: AnnotationHolder) {
        val file = element as? SconFile ?: return
        try {
            val document = file.parseSconDocument()
            file.resolveSconDocument(document)
        } catch (err: SconException) {
            val range = err.error.span?.toTextRange(file) ?: file.textRange
            holder.newAnnotation(HighlightSeverity.ERROR, "${err.error.code}: ${err.error.message}")
                .range(range)
                .create()
        }
    }
}
