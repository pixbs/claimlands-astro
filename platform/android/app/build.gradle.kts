plugins {
    id("com.android.application")
}

val workspaceRoot = rootProject.projectDir.resolve("../..").canonicalFile
val pinnedNdk = "27.2.12479018"
val rustAbis = providers.gradleProperty("rustAbis").orElse("arm64-v8a,x86_64")
    .get().split(',').map(String::trim)
require(rustAbis.isNotEmpty() && rustAbis.all { it in setOf("arm64-v8a", "x86_64") }) {
    "rustAbis must contain arm64-v8a and/or x86_64"
}

android {
    namespace = "net.pixbs.claimlands"
    compileSdk = 37
    buildToolsVersion = "37.0.0"
    ndkVersion = pinnedNdk

    defaultConfig {
        applicationId = "net.pixbs.claimlands"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += rustAbis }
    }
    buildTypes {
        debug { isDebuggable = true }
        release { isMinifyEnabled = false }
    }
    packaging {
        jniLibs {
            useLegacyPackaging = false
            // Retain symbols for actionable native crash reports in this foundation.
            keepDebugSymbols += "**/libclaimlands_game.so"
        }
    }
    lint {
        abortOnError = true
        warningsAsErrors = true
        textReport = true
        textOutput = file("build/reports/lint-results.txt")
    }
}

abstract class BuildRustTask : Exec() {
    @get:OutputDirectory
    abstract val nativeLibraries: DirectoryProperty
}

androidComponents.onVariants { variant ->
    val capitalized = variant.name.replaceFirstChar(Char::uppercaseChar)
    val nativeLibraries = layout.buildDirectory.dir("rust/${variant.name}/jniLibs")
    val rustBuild = tasks.register<BuildRustTask>("buildRust$capitalized") {
        this.nativeLibraries.set(nativeLibraries)
        group = "build"
        description = "Build the shared Rust game for ${variant.name} Android ABIs"
        workingDir = workspaceRoot
        // Cargo owns incremental compilation. Always invoke it so transitive Rust
        // source or feature changes cannot leave stale packaged shared libraries.
        outputs.upToDateWhen { false }
        val args = mutableListOf("cargo", "ndk", "--platform", "26")
        rustAbis.forEach { args += listOf("--target", it) }
        args += listOf("-o", nativeLibraries.get().asFile.absolutePath,
            "build", "--locked", "--package", "claimlands-game", "--lib")
        if (variant.buildType == "release") args += "--release"
        commandLine(args)
        doFirst {
            val sdkRoot = System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
                ?: error("Set ANDROID_HOME to the Android SDK directory")
            val ndkDirectory = file("$sdkRoot/ndk/$pinnedNdk")
            check(ndkDirectory.resolve("source.properties").readText()
                .contains("Pkg.Revision = $pinnedNdk")) { "Install pinned NDK $pinnedNdk" }
            environment("ANDROID_NDK_HOME", ndkDirectory.absolutePath)
            environment("ANDROID_NDK_ROOT", ndkDirectory.absolutePath)
            val flags = System.getenv("RUSTFLAGS").orEmpty()
            environment("RUSTFLAGS", "$flags -C link-arg=-Wl,-z,max-page-size=16384")
        }
    }
    // The generated-source API wires packaging to the producer task and marks
    // these libraries as generated, without depending on internal AGP task names.
    checkNotNull(variant.sources.jniLibs).addGeneratedSourceDirectory(rustBuild) {
        it.nativeLibraries
    }
}
