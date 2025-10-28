plugins {
    kotlin("jvm") version "2.0.0"
}

group = "org.aria"
version = "0.1.0"

repositories {
    mavenCentral()
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(17))
    }
}

tasks.jar {
    archiveBaseName.set("classlib")
    destinationDirectory.set(file("$buildDir/libs"))
    from(sourceSets.main.get().output)
    manifest {
        attributes["Implementation-Title"] = "Aria Class Library"
        attributes["Implementation-Version"] = version
    }
}

tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    kotlinOptions.jvmTarget = "17"
}
