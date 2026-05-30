package io.github.realmlabs.scon.idea

import com.intellij.openapi.fileTypes.LanguageFileType
import javax.swing.Icon

class SconFileType private constructor() : LanguageFileType(SconLanguage) {
    override fun getName(): String = "SCON"
    override fun getDescription(): String = "SCON configuration file"
    override fun getDefaultExtension(): String = "scon"
    override fun getIcon(): Icon? = null

    companion object {
        @JvmField
        val INSTANCE = SconFileType()
    }
}
