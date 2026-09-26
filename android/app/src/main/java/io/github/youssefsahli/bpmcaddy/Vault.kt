package io.github.youssefsahli.bpmcaddy

import android.content.Context
import android.os.Build
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import java.security.KeyStore
import java.security.SecureRandom
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * La clé de la base du téléphone.
 *
 * La base est chiffrée (SQLCipher) sous une phrase tirée au hasard la
 * première fois — personne ne la tape. Cette phrase est elle-même
 * chiffrée sous une clé AES du **Keystore Android**, qui ne sort jamais du
 * matériel du téléphone et ne sert qu'après que la personne s'est
 * authentifiée (empreinte, visage ou code de l'écran de verrouillage).
 * Un téléphone perdu, verrouillé, ne rend donc ni la base ni sa clé.
 */
class Vault(private val context: Context) {
    private val prefs = context.getSharedPreferences("coffre", Context.MODE_PRIVATE)

    /** La phrase de la base. À appeler juste après une authentification réussie. */
    fun passphrase(): String {
        val sealed = prefs.getString(SEALED, null)
        val iv = prefs.getString(IV, null)
        if (sealed != null && iv != null) {
            val cipher = Cipher.getInstance(TRANSFORM)
            cipher.init(Cipher.DECRYPT_MODE, key(), GCMParameterSpec(128, decode(iv)))
            return String(cipher.doFinal(decode(sealed)), Charsets.UTF_8)
        }
        val fresh = ByteArray(32).also { SecureRandom().nextBytes(it) }
            .joinToString("") { "%02x".format(it) }
        val cipher = Cipher.getInstance(TRANSFORM)
        cipher.init(Cipher.ENCRYPT_MODE, key())
        val out = cipher.doFinal(fresh.toByteArray(Charsets.UTF_8))
        prefs.edit()
            .putString(SEALED, encode(out))
            .putString(IV, encode(cipher.iv))
            .apply()
        return fresh
    }

    private fun key(): SecretKey {
        val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
        (store.getKey(ALIAS, null) as? SecretKey)?.let { return it }
        val spec = KeyGenParameterSpec.Builder(
            ALIAS,
            KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
        )
            .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
            .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
            .setKeySize(256)
            .setUserAuthenticationRequired(true)
            .apply {
                // Utilisable une minute après l'authentification : le temps
                // d'ouvrir la base, pas davantage.
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
                    setUserAuthenticationParameters(
                        VALIDITY_SECONDS,
                        KeyProperties.AUTH_BIOMETRIC_STRONG or KeyProperties.AUTH_DEVICE_CREDENTIAL,
                    )
                } else {
                    @Suppress("DEPRECATION")
                    setUserAuthenticationValidityDurationSeconds(VALIDITY_SECONDS)
                }
            }
            .build()
        return KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore")
            .apply { init(spec) }
            .generateKey()
    }

    private fun encode(b: ByteArray) = Base64.encodeToString(b, Base64.NO_WRAP)
    private fun decode(s: String) = Base64.decode(s, Base64.NO_WRAP)

    companion object {
        private const val ALIAS = "bpm-caddy-base"
        private const val TRANSFORM = "AES/GCM/NoPadding"
        private const val SEALED = "phrase"
        private const val IV = "iv"
        private const val VALIDITY_SECONDS = 60
    }
}
