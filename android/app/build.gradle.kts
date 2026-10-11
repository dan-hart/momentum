// SPDX-License-Identifier: GPL-3.0-or-later
import groovy.json.JsonOutput
import org.gradle.api.artifacts.component.ModuleComponentIdentifier
import org.gradle.api.artifacts.result.ResolvedArtifactResult
import org.gradle.maven.MavenModule
import org.gradle.maven.MavenPomArtifact
import org.gradle.api.artifacts.transform.InputArtifact
import org.gradle.api.artifacts.transform.TransformAction
import org.gradle.api.artifacts.transform.TransformOutputs
import org.gradle.api.artifacts.transform.TransformParameters
import org.gradle.api.artifacts.type.ArtifactTypeDefinition
import org.gradle.api.attributes.Attribute
import org.gradle.process.ExecOperations
import javax.inject.Inject

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.compose)
}

abstract class GraphicsPathSourceTransform : TransformAction<GraphicsPathSourceTransform.Parameters> {
    interface Parameters : TransformParameters {
        @get:InputFile @get:PathSensitive(PathSensitivity.RELATIVE)
        val script: RegularFileProperty
        @get:InputFile @get:PathSensitive(PathSensitivity.RELATIVE)
        val toolchain: RegularFileProperty
        @get:InputFile @get:PathSensitive(PathSensitivity.RELATIVE)
        val coreScript: RegularFileProperty
        @get:InputDirectory @get:PathSensitive(PathSensitivity.RELATIVE)
        val nativeSources: DirectoryProperty
    }

    @get:InputArtifact @get:PathSensitive(PathSensitivity.NAME_ONLY)
    abstract val inputArtifact: Provider<FileSystemLocation>
    @get:Inject abstract val execOperations: ExecOperations

    override fun transform(outputs: TransformOutputs) {
        val original = inputArtifact.get().asFile
        if (!original.name.startsWith("graphics-path-")) {
            outputs.file(original)
            return
        }
        val rebuilt = outputs.file("graphics-path-1.1.0-source-jni.aar")
        val native = rebuilt.parentFile.resolve("source-jni")
        // Compile inside this transform's workspace: no race with preBuild or
        // APK merging, and no dependence on an existing generated binary.
        execOperations.exec {
            commandLine("python3", parameters.script.get().asFile.absolutePath,
                "--output-dir", native.absolutePath)
        }.assertNormalExitValue()
        execOperations.exec {
            commandLine("python3", parameters.script.get().asFile.absolutePath,
                "--output-dir", native.absolutePath, "--repack-aar", original.absolutePath,
                "--output-aar", rebuilt.absolutePath)
        }.assertNormalExitValue()
    }
}

val repositoryRoot = rootProject.layout.projectDirectory.dir("..")
val sourceNative = Attribute.of("com.codedbydan.momentum.source-native", Boolean::class.javaObjectType)
dependencies {
    attributesSchema { attribute(sourceNative) }
    artifactTypes.maybeCreate("aar").attributes.attribute(sourceNative, false)
    registerTransform(GraphicsPathSourceTransform::class) {
        from.attribute(sourceNative, false).attribute(ArtifactTypeDefinition.ARTIFACT_TYPE_ATTRIBUTE, "aar")
        to.attribute(sourceNative, true).attribute(ArtifactTypeDefinition.ARTIFACT_TYPE_ATTRIBUTE, "aar")
        parameters {
            script.set(repositoryRoot.file("android/scripts/build_graphics_path.py"))
            toolchain.set(repositoryRoot.file("android/toolchain.json"))
            coreScript.set(repositoryRoot.file("android/scripts/build_core.py"))
            nativeSources.set(repositoryRoot.dir("android/vendor/graphics-path"))
        }
    }
}
configurations.configureEach {
    attributes.attribute(sourceNative, true)
}
// Cargo is the release source of truth; increasing semver produces increasing codes.
val workspacePackage = repositoryRoot.file("Cargo.toml").asFile.readText()
    .substringAfter("[workspace.package]").substringBefore("\n[")
val workspaceVersion = Regex("""(?m)^version\s*=\s*"(\d+\.\d+\.\d+)"\s*$""")
    .find(workspacePackage)?.groupValues?.get(1) ?: error("Missing stable workspace version")
val versionParts = workspaceVersion.split('.').map(String::toInt)
require(versionParts[0] in 0..2099 && versionParts[1] in 0..999 && versionParts[2] in 0..999) {
    "Workspace version exceeds Android's monotonic versionCode scheme"
}
val workspaceVersionCode = versionParts[0] * 1_000_000 + versionParts[1] * 1_000 + versionParts[2]
require(workspaceVersionCode > 0)
val buildRustCore = tasks.register<Exec>("buildRustCore") {
    group = "build"
    description = "Build pinned Rust sources and regenerate Kotlin UniFFI bindings"
    workingDir(repositoryRoot)
    commandLine("python3", "android/scripts/build_core.py", "--profile",
        providers.gradleProperty("momentumRustProfile").getOrElse("release"))
}

val generateLicenseNotices = tasks.register("generateLicenseNotices") {
    group = "build"
    description = "Package full dependency licenses/notices and exact runtime coordinates"
    dependsOn(buildRustCore)
    doLast {
        val runtime = configurations.getByName("releaseRuntimeClasspath")
        val componentIds = runtime.incoming.resolutionResult.allComponents.map { it.id }
            .filterIsInstance<ModuleComponentIdentifier>()
        val pomResults = dependencies.createArtifactResolutionQuery()
            .forComponents(componentIds)
            .withArtifacts(MavenModule::class.java, MavenPomArtifact::class.java).execute()
        val poms = pomResults.resolvedComponents.associate { component ->
            component.id.displayName to component.getArtifacts(MavenPomArtifact::class.java)
                .filterIsInstance<ResolvedArtifactResult>().single().file.absolutePath
        }
        val artifacts = runtime.resolvedConfiguration.resolvedArtifacts.map { artifact ->
            val id = artifact.moduleVersion.id.toString()
            mapOf("id" to id, "path" to artifact.file.absolutePath,
                "pom" to (poms[id] ?: error("Missing public POM for $id")))
        }
        val input = layout.buildDirectory.file("notices/gradle-runtime.json").get().asFile
        input.parentFile.mkdirs()
        input.writeText(JsonOutput.toJson(artifacts))
        providers.exec {
            workingDir(repositoryRoot)
            commandLine("python3", "android/scripts/package_notices.py", "--gradle-input", input.absolutePath)
        }.result.get().assertNormalExitValue()
    }
}

android {
    namespace = "com.codedbydan.momentum"
    compileSdk = 37
    buildToolsVersion = "37.0.0"
    ndkVersion = "27.2.12479018"
    defaultConfig {
        applicationId = "com.codedbydan.momentum"
        minSdk = 26
        targetSdk = 37
        versionCode = workspaceVersionCode
        versionName = workspaceVersion
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    // No private key is required to build an unsigned release for independent stores.
    val signingNames = listOf("MOMENTUM_KEYSTORE", "MOMENTUM_KEYSTORE_PASSWORD", "MOMENTUM_KEY_ALIAS", "MOMENTUM_KEY_PASSWORD")
    val signingValues = signingNames.map { providers.environmentVariable(it).orNull }
    require(signingValues.all { it == null } || signingValues.all { !it.isNullOrBlank() }) {
        "Release signing requires all four MOMENTUM_KEYSTORE/MOMENTUM_KEYSTORE_PASSWORD/MOMENTUM_KEY_ALIAS/MOMENTUM_KEY_PASSWORD environment variables"
    }
    if (signingValues.all { it != null }) {
        signingConfigs.create("distribution") {
            storeFile = file(signingValues[0]!!)
            storePassword = signingValues[1]
            keyAlias = signingValues[2]
            keyPassword = signingValues[3]
        }
    }
    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("distribution")
        }
    }
    sourceSets.named("main") {
        kotlin.directories += "build/generated/uniffi"
        jniLibs.directories += "build/generated/jniLibs"
        assets.directories += "build/generated/notices"
    }
    buildFeatures { compose = true }
    androidResources {
        generateLocaleConfig = true
        localeFilters += listOf("en", "de")
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
        isCoreLibraryDesugaringEnabled = true
    }
    packaging {
        jniLibs.useLegacyPackaging = false
        resources.merges += setOf("META-INF/AL2.0", "META-INF/LGPL2.1")
    }
    testOptions.unitTests.all {
        it.systemProperty("jna.library.path", repositoryRoot.dir("android/rust-target/debug").asFile.absolutePath)
    }
    lint { abortOnError = true }
}

tasks.named("preBuild").configure { dependsOn(buildRustCore) }
tasks.matching { it.name.startsWith("compile") && it.name.endsWith("Kotlin") }.configureEach { dependsOn(buildRustCore) }
tasks.matching { it.name.startsWith("merge") && it.name.endsWith("Assets") }.configureEach { dependsOn(generateLicenseNotices) }

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.compose.ui)
    // Published classes stay byte-identical; JNI is rebuilt from the pinned
    // matching Apache-licensed release sources with both 16 KB linker flags.
    implementation("androidx.graphics:graphics-path:1.1.0") {
        version { strictly("1.1.0") }
    }
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.material3.adaptive.navigation)
    implementation(libs.androidx.compose.icons)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.androidx.datastore)
    implementation(libs.androidx.work)
    implementation("net.java.dev.jna:jna:${libs.versions.jna.get()}@aar")
    coreLibraryDesugaring(libs.desugar)
    testImplementation(libs.junit)
    testImplementation(libs.kotlinx.coroutines.test)
    testRuntimeOnly(libs.jna)
    androidTestImplementation(platform(libs.androidx.compose.bom))
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.androidx.test.junit)
    // Compose's legacy Espresso transitive version uses removed hidden input APIs.
    androidTestImplementation(libs.androidx.test.espresso)
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
    debugImplementation(libs.androidx.compose.ui.tooling)
    debugImplementation(libs.androidx.compose.ui.test.manifest)
}
