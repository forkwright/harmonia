import java.util.Properties
import java.io.FileInputStream

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.compose)
    alias(libs.plugins.ksp)
    alias(libs.plugins.hilt)
    id("org.owasp.dependencycheck")
    id("jacoco")
}

android {
    namespace = "app.akroasis"
    compileSdk = 35

    defaultConfig {
        applicationId = "app.akroasis"
        minSdk = 29
        targetSdk = 35
        versionCode = 1
        versionName = "0.1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"

        // Load API credentials from local.properties or environment
        val properties = Properties()
        val localPropertiesFile = rootProject.file("local.properties")
        if (localPropertiesFile.exists()) {
            properties.load(FileInputStream(localPropertiesFile))
        }

        buildConfigField(
            "String",
            "LASTFM_API_KEY",
            "\"${properties.getProperty("lastfm.api.key", System.getenv("LASTFM_API_KEY") ?: "")}\""
        )
        buildConfigField(
            "String",
            "LASTFM_API_SECRET",
            "\"${properties.getProperty("lastfm.api.secret", System.getenv("LASTFM_API_SECRET") ?: "")}\""
        )
    }

    signingConfigs {
        create("release") {
            val properties = Properties()
            val localPropertiesFile = rootProject.file("local.properties")
            if (localPropertiesFile.exists()) {
                properties.load(FileInputStream(localPropertiesFile))
            }

            val keystoreFile = properties.getProperty("KEYSTORE_FILE")
                ?: System.getenv("KEYSTORE_FILE")
            val keystorePassword = properties.getProperty("KEYSTORE_PASSWORD")
                ?: System.getenv("KEYSTORE_PASSWORD")
            val keyAlias = properties.getProperty("KEY_ALIAS")
                ?: System.getenv("KEY_ALIAS")
            val keyPassword = properties.getProperty("KEY_PASSWORD")
                ?: System.getenv("KEY_PASSWORD")

            if (keystoreFile != null && keystorePassword != null && keyAlias != null && keyPassword != null) {
                storeFile = file(keystoreFile)
                storePassword = keystorePassword
                this.keyAlias = keyAlias
                this.keyPassword = keyPassword
            }
        }
    }

    buildTypes {
        debug {
            enableUnitTestCoverage = true
        }

        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )

            // Sign release builds if keystore configured
            val releaseSigningConfig = signingConfigs.getByName("release")
            if (releaseSigningConfig.storeFile != null) {
                signingConfig = releaseSigningConfig
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
        isCoreLibraryDesugaringEnabled = true
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    lint {
        // Disable problematic lint check causing IncompatibleClassChangeError
        disable += "NullSafeMutableLiveData"

        // Continue on errors for now
        abortOnError = false
    }

    testOptions {
        unitTests.all {
            it.maxHeapSize = "2048m"
        }
    }
}

dependencies {
    // Core
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.lifecycle.runtime.ktx)
    implementation(libs.androidx.activity.compose)

    // Logging
    implementation(libs.timber)

    // Compose
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.graphics)
    implementation(libs.androidx.compose.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.material)
    implementation(libs.androidx.compose.material.icons.extended)

    // Security
    implementation(libs.androidx.security.crypto)

    // Media
    implementation(libs.androidx.media)

    // Hilt DI
    implementation(libs.hilt.android)
    implementation(libs.hilt.navigation.compose)
    ksp(libs.hilt.compiler)

    // Networking
    implementation(libs.retrofit)
    implementation(libs.retrofit.gson)
    implementation(libs.okhttp)
    implementation(libs.okhttp.logging)

    // Room Database
    implementation(libs.room.runtime)
    implementation(libs.room.ktx)
    ksp(libs.room.compiler)

    // Image Loading
    implementation(libs.coil.compose)

    // EPUB Reader - Readium Kotlin Toolkit
    implementation(libs.readium.shared)
    implementation(libs.readium.streamer)
    implementation(libs.readium.navigator)

    // Drag and Drop
    implementation(libs.reorderable)

    // Core Library Desugaring (required for Readium)
    coreLibraryDesugaring(libs.desugar.jdk.libs)

    // Testing
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.mockito.kotlin)
    testImplementation(libs.mockito.core)
    testImplementation(libs.turbine)
    testImplementation(libs.androidx.arch.core.testing)
    testImplementation(libs.robolectric)
}

ksp {
    arg("correctErrorTypes", "true")
}

dependencyCheck {
    formats = listOf("HTML", "JSON")
    failBuildOnCVSS = 7.0f
    suppressionFile = "dependency-check-suppressions.xml"
    nvd.apiKey = System.getenv("NVD_API_KEY") ?: ""
}

// Jacoco configuration for code coverage
android.applicationVariants.all {
    val variant = this
    val testTaskName = "test${variant.name.replaceFirstChar { it.uppercase() }}UnitTest"

    tasks.register<JacocoReport>("${testTaskName}Coverage") {
        dependsOn(testTaskName)
        group = "Reporting"
        description = "Generate Jacoco coverage reports for ${variant.name} variant."

        reports {
            xml.required.set(true)
            html.required.set(true)
        }

        val fileFilter = listOf(
            "**/R.class",
            "**/R$*.class",
            "**/BuildConfig.*",
            "**/Manifest*.*",
            "**/*Test*.*",
            "android/**/*.*",
            "**/*_Hilt*.*",
            "**/*_Factory*.*",
            "**/*_MembersInjector*.*",
            "**/*Module_Provide*Factory*.*"
        )

        val javaTree = fileTree("${layout.buildDirectory.asFile.get()}/intermediates/javac/${variant.name}/classes") {
            exclude(fileFilter)
        }
        val kotlinTree = fileTree("${layout.buildDirectory.asFile.get()}/tmp/kotlin-classes/${variant.name}") {
            exclude(fileFilter)
        }

        classDirectories.setFrom(files(listOf(javaTree, kotlinTree)))
        executionData.setFrom(fileTree(layout.buildDirectory.asFile.get()) {
            include("**/*.exec", "**/*.ec")
        })
        sourceDirectories.setFrom(files(listOf(
            "src/main/java",
            "src/main/kotlin",
            "src/${variant.name}/java",
            "src/${variant.name}/kotlin"
        )))
    }
}
