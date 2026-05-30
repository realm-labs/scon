import org.jetbrains.intellij.platform.gradle.extensions.intellijPlatform

pluginManagement {
    repositories {
        gradlePluginPortal()
        mavenCentral()
    }
}

plugins {
    id("org.jetbrains.intellij.platform.settings") version "2.16.0"
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        mavenCentral()
        intellijPlatform {
            defaultRepositories()
        }
    }
}

rootProject.name = "scon"

include(":scon-core")
include(":scon-kotlinx-serialization")
include(":idea-plugin")

project(":scon-core").projectDir = file("scon-core")
project(":scon-kotlinx-serialization").projectDir = file("scon-kotlinx-serialization")
project(":idea-plugin").projectDir = file("../editors/idea")
