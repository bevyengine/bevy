plugins {
    alias(libs.plugins.android.application)
}

kotlin {
    compilerOptions {
        languageVersion = org.jetbrains.kotlin.gradle.dsl.KotlinVersion.KOTLIN_2_3
        jvmToolchain(8)
    }
}

android {
    namespace = "org.bevyengine.example"
    compileSdk = 36

    // https://developer.android.com/reference/tools/gradle-api/9.1/com/android/build/api/dsl/DefaultConfig
    defaultConfig {
        applicationId = "org.bevyengine.example"
        minSdk = 31
        targetSdk = 36
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
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    // https://developer.android.com/reference/tools/gradle-api/9.1/com/android/build/api/dsl/BuildFeatures
    buildFeatures {
        prefab = true
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
