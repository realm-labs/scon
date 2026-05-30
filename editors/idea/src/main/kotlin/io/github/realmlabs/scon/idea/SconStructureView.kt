package io.github.realmlabs.scon.idea

import com.intellij.ide.structureView.StructureViewBuilder
import com.intellij.ide.structureView.StructureViewModel
import com.intellij.ide.structureView.StructureViewModelBase
import com.intellij.ide.structureView.TreeBasedStructureViewBuilder
import com.intellij.ide.structureView.impl.common.PsiTreeElementBase
import com.intellij.ide.util.treeView.smartTree.Sorter
import com.intellij.navigation.ItemPresentation
import com.intellij.openapi.editor.Editor
import com.intellij.psi.PsiFile
import com.intellij.psi.PsiNamedElement
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiManager
import com.intellij.psi.impl.FakePsiElement
import javax.swing.Icon

class SconStructureViewFactory : com.intellij.lang.PsiStructureViewFactory {
    override fun getStructureViewBuilder(psiFile: PsiFile): StructureViewBuilder? {
        val file = psiFile as? SconFile ?: return null
        return object : TreeBasedStructureViewBuilder() {
            override fun createStructureViewModel(editor: Editor?): StructureViewModel =
                StructureViewModelBase(file, editor, SconStructureElement(file, null))
                    .withSorters(Sorter.ALPHA_SORTER)
        }
    }
}

private class SconStructureElement(
    private val file: SconFile,
    private val definition: SconDefinition?,
) : PsiTreeElementBase<PsiElement>(definition?.let { SconNavigationElement(file, it) } ?: file) {
    override fun getChildrenBase(): Collection<SconStructureElement> {
        if (definition != null) return emptyList()
        return runCatching { file.parseSconDocument().collectDefinitionPaths() }
            .getOrDefault(emptyList())
            .map { SconStructureElement(file, it) }
    }

    override fun getPresentableText(): String =
        definition?.dotted ?: file.name
}

private class SconNavigationElement(
    private val file: SconFile,
    private val definition: SconDefinition,
) : FakePsiElement(), PsiNamedElement {
    override fun getParent(): PsiElement = file
    override fun getContainingFile(): PsiFile = file
    override fun getName(): String = definition.dotted
    override fun setName(name: String): PsiElement = this
    override fun canNavigate(): Boolean = true
    override fun canNavigateToSource(): Boolean = true
    override fun navigate(requestFocus: Boolean) {
        file.virtualFile?.let {
            com.intellij.openapi.fileEditor.OpenFileDescriptor(file.project, it, definition.span.start).navigate(requestFocus)
        }
    }

    override fun getPresentation(): ItemPresentation =
        object : ItemPresentation {
            override fun getPresentableText(): String = definition.dotted
            override fun getLocationString(): String? = null
            override fun getIcon(unused: Boolean): Icon? = SconFileType.INSTANCE.icon
        }
}
