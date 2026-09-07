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
    compileSdk = 35
    buildToolsVersion = "35.0.0"
    ndkVersion = pinnedNdk

    defaultConfig {
        applicationId = "net.pixbs.claimlands"
        minSdk = 26
        targetSdk = 35
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
    }
}

for (variant in listOf("debug", "release")) {
    val capitalized = variant.replaceFirstChar(Char::uppercaseChar)
    val nativeLibraries = layout.buildDirectory.dir("rust/$variant/jniLibs")
    android.sourceSets.getByName(variant).jniLibs.srcDir(nativeLibraries)
    val rustBuild = tasks.register<Exec>("buildRust$capitalized") {
        group = "build"
        description = "Build the shared Rust game for $variant Android ABIs"
        workingDir = workspaceRoot
        // Cargo owns incremental compilation. Always invoke it so transitive Rust
        // source or feature changes cannot leave stale packaged shared libraries.
        val args = mutableListOf("cargo", "ndk", "--platform", "26")
        rustAbis.forEach { args += listOf("--target", it) }
        args += listOf("-o", nativeLibraries.get().asFile.absolutePath,
            "build", "--locked", "--package", "claimlands-game", "--lib")
        if (variant == "release") args += "--release"
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
    tasks.configureEach {
        if (name == "merge${capitalized}JniLibFolders") dependsOn(rustBuild)
    }
}
