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

tasks.named<Jar>("jar") {
    archiveBaseName.set("classlib")
    archiveVersion.set("")
    archiveFileName.set("classlib.jar")
    destinationDirectory.set(file("$buildDir/libs"))
    from(sourceSets.main.get().output)
    manifest {
        attributes(
            "Implementation-Title" to "Aria Class Library",
            "Implementation-Version" to version
        )
    }
}


tasks.withType<org.jetbrains.kotlin.gradle.tasks.KotlinCompile> {
    kotlinOptions.jvmTarget = "17"
}
