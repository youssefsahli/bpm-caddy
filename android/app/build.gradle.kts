import java.util.Properties

// Le compagnon Android de BPM-Caddy. L'interface est ici (Compose) ; tout
// le reste — la base SQLCipher, la synchronisation, les fiches — est la
// bibliothèque Rust de l'application, construite par `../build-rust.sh`
// dans `src/main/jniLibs` avec ses liaisons Kotlin (UniFFI).
plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

// La clé de signature du Play Store ne vit pas dans le dépôt : un fichier
// `keystore.properties` à côté de ce dossier (storeFile, storePassword,
// keyAlias, keyPassword), ou les mêmes noms en variables d'environnement
// (BPM_ANDROID_STORE_FILE…) pour la publication. Sans eux, la version
// « release » se construit non signée.
val signing = Properties().apply {
    val f = rootProject.file("keystore.properties")
    if (f.exists()) f.inputStream().use { load(it) }
    System.getenv("BPM_ANDROID_STORE_FILE")?.let { setProperty("storeFile", it) }
    System.getenv("BPM_ANDROID_STORE_PASSWORD")?.let { setProperty("storePassword", it) }
    System.getenv("BPM_ANDROID_KEY_ALIAS")?.let { setProperty("keyAlias", it) }
    System.getenv("BPM_ANDROID_KEY_PASSWORD")?.let { setProperty("keyPassword", it) }
}

android {
    namespace = "io.github.youssefsahli.bpmcaddy"
    compileSdk = 36
    buildToolsVersion = "36.0.0"

    defaultConfig {
        applicationId = "io.github.youssefsahli.bpmcaddy"
        minSdk = 26
        targetSdk = 36
        // Suit la version de l'application de bureau : 0.350.0 → 350000.
        versionCode = 354000
        versionName = "0.354.0"
    }

    signingConfigs {
        if (signing.getProperty("storeFile") != null) {
            create("play") {
                storeFile = file(signing.getProperty("storeFile"))
                storePassword = signing.getProperty("storePassword")
                keyAlias = signing.getProperty("keyAlias")
                keyPassword = signing.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        release {
            signingConfigs.findByName("play")?.let { signingConfig = it }
            isMinifyEnabled = true
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro",
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

// **La bibliothèque Rust est reconstruite avec l'application** : les
// libellés (`assets/strings.fr.toml`) y sont compilés, et un APK construit
// sur une bibliothèque d'avant montrait des clés à la place du français.
// Gradle ne relance `build-rust.sh` que si une source a changé.
val buildRust by tasks.registering(Exec::class) {
    val root = rootProject.projectDir.parentFile
    workingDir = rootProject.projectDir
    commandLine("./build-rust.sh")
    inputs.dir(File(root, "src"))
    inputs.dir(File(root, "sync/src"))
    inputs.dir(File(root, "mobile/src"))
    inputs.file(File(root, "assets/strings.fr.toml"))
    inputs.file(File(root, "Cargo.lock"))
    outputs.dir(file("src/main/jniLibs"))
    outputs.dir(file("src/main/java/uniffi"))
}
tasks.named("preBuild") { dependsOn(buildRust) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2025.06.01")
    implementation(composeBom)
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-core")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.9.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.9.1")
    implementation("androidx.biometric:biometric:1.1.0")
    implementation("androidx.documentfile:documentfile:1.1.0")
    // Le lecteur de codes de Google : il ouvre sa propre caméra, sans que
    // l'application demande l'autorisation de la caméra.
    implementation("com.google.android.gms:play-services-code-scanner:16.1.0")
    implementation("androidx.fragment:fragment-ktx:1.8.8")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    // UniFFI's Kotlin bindings call the Rust library through JNA.
    implementation("net.java.dev.jna:jna:5.17.0@aar")
    debugImplementation("androidx.compose.ui:ui-tooling")
}
