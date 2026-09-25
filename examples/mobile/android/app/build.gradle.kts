plugins {
    alias(libs.plugins.android.application)
}

java {
    // https://docs.gradle.org/current/userguide/toolchains.html
    toolchain {
        languageVersion = JavaLanguageVersion.of(17)
    }
}

kotlin {
    // https://kotlinlang.org/docs/gradle-compiler-options.html#all-compiler-options
    compilerOptions {
        languageVersion = org.jetbrains.kotlin.gradle.dsl.KotlinVersion.KOTLIN_2_3
        jvmToolchain(17)
    }
}

android {
    namespace = "org.bevyengine.example"
    compileSdk = 37

    // https://developer.android.com/reference/tools/gradle-api/9.1/com/android/build/api/dsl/DefaultConfig
    defaultConfig {
        applicationId = "org.bevyengine.example"
        // NOTE: `minSdk` is 26 because this is the minimum supported by `bevy_audio`
        minSdk = 26
        targetSdk = 37
        // NOTE: Increase by 1 on each release
        versionCode = 1
        // NOTE: Update with full semantic version on each release
        versionName = "0.0.0"
        // https://developer.android.com/reference/tools/gradle-api/9.1/com/android/build/api/dsl/Ndk
        ndk {
            abiFilters.addAll(listOf("arm64-v8a", "armeabi-v7a", "x86_64"))
        }
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }
    // https://developer.android.com/reference/tools/gradle-api/9.1/com/android/build/api/dsl/BuildType
    buildTypes {
        getByName("release") {
            // https://developer.android.com/topic/performance/app-optimization/enable-app-optimization
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
        }
    }
    // https://developer.android.com/reference/tools/gradle-api/9.1/com/android/build/api/dsl/CompileOptions
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    // https://developer.android.com/reference/tools/gradle-api/9.1/com/android/build/api/dsl/AndroidSourceSet
    sourceSets {
        getByName("main") {
            assets {
                directories += "../../../../assets"
            }
        }
    }
}

dependencies {
    implementation(libs.appcompat)
    implementation(libs.core)
    implementation(libs.material)
    implementation(libs.games.activity)
    implementation(libs.core.ktx)
}
