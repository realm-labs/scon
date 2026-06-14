plugins {
    kotlin("jvm")
}

dependencies {
    testImplementation("com.code-intelligence:jazzer-junit:0.30.0")
    testImplementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.11.0")
    testImplementation(kotlin("test"))
}

kotlin {
    jvmToolchain(17)
}

tasks.test {
    useJUnitPlatform()
}
