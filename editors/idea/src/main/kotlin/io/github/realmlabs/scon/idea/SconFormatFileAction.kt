package io.github.realmlabs.scon.idea

import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.fileEditor.FileDocumentManager
import io.github.realmlabs.scon.SconException
import io.github.realmlabs.scon.SconParseOptions
import io.github.realmlabs.scon.formatSource

class SconFormatFileAction : AnAction() {
    override fun update(event: AnActionEvent) {
        val file = event.getData(CommonDataKeys.PSI_FILE)
        event.presentation.isEnabledAndVisible = file is SconFile
    }

    override fun actionPerformed(event: AnActionEvent) {
        val file = event.getData(CommonDataKeys.PSI_FILE) as? SconFile ?: return
        val document = FileDocumentManager.getInstance().getDocument(file.virtualFile ?: return) ?: return
        val formatted = try {
            formatSource(
                document.text,
                SconParseOptions(
                    sourceName = file.virtualFile.path,
                    sourcePath = file.sconSourcePath(),
                ),
            )
        } catch (_: SconException) {
            return
        }
        if (formatted == document.text) return
        WriteCommandAction.runWriteCommandAction(file.project, "Format SCON File", null, Runnable {
            document.setText(formatted)
        }, file)
    }
}
