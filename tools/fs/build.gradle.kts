plugins {
    kotlin("jvm") version "2.0.0"
}

group = "org.aria.tools"
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
    archiveBaseName.set("jrt-fs")
    archiveVersion.set("")
    archiveFileName.set("jrt-fs.jar")
    destinationDirectory.set(file("$buildDir/libs"))
    from(sourceSets.main.get().output)
    manifest {
        attributes(
            "Implementation-Title" to "Aria JRT Filesystem Provider",
            "Implementation-Version" to version
        )
    }
}