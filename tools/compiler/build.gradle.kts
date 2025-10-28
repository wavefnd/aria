plugins {
    kotlin("jvm") version "2.0.0"
    application
}

group = "org.aria.tools"
version = "0.1.0"

repositories {
    mavenCentral()
}

application {
    mainClass.set("org.aria.compiler.MainKt")
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    kotlinOptions.jvmTarget = "17"
}
