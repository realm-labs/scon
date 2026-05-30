import org.jetbrains.intellij.platform.gradle.TestFrameworkType

plugins {
    kotlin("jvm")
    id("org.jetbrains.intellij.platform")
}

dependencies {
    implementation(project(":kotlin:scon-core"))

    intellijPlatform {
        intellijIdeaCommunity("2024.2.5")
        testFramework(TestFrameworkType.Platform)
        pluginVerifier()
        zipSigner()
    }

    testImplementation(kotlin("test"))
}

kotlin {
    jvmToolchain(17)
}

intellijPlatform {
    pluginConfiguration {
        name = "SCON"
        id = "io.github.realmlabs.scon"
        version = project.version.toString()

        ideaVersion {
            sinceBuild = "242"
        }

        vendor {
            name = "Realm Labs"
            url = "https://github.com/realm-labs"
        }

        description = """
            Native SCON language support for IntelliJ-based IDEs: syntax
            highlighting, diagnostics, completion, navigation, documentation,
            structure view, and formatting backed by the shared Kotlin SCON
            implementation.
        """.trimIndent()

        changeNotes = """
            Add native SCON language support backed by kotlin/scon-core.
        """.trimIndent()
    }

    signing {
        certificateChain = providers.environmentVariable("JETBRAINS_CERTIFICATE_CHAIN")
        privateKey = providers.environmentVariable("JETBRAINS_PRIVATE_KEY")
        password = providers.environmentVariable("JETBRAINS_PRIVATE_KEY_PASSWORD")
    }

    publishing {
        token = providers.environmentVariable("JETBRAINS_MARKETPLACE_TOKEN")
    }

    pluginVerification {
        ides {
            current()
        }
    }
}

tasks.test {
    useJUnitPlatform()
}
