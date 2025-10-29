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
    archiveVersion.set("") // ✅ 버전 제거
    archiveFileName.set("classlib.jar") // ✅ 파일 이름 고정
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
