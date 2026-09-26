package io.github.youssefsahli.bpmcaddy

import android.content.Context
import android.net.wifi.WifiManager
import android.os.Build
import android.os.Bundle
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.biometric.BiometricManager.Authenticators.BIOMETRIC_STRONG
import androidx.biometric.BiometricManager.Authenticators.BIOMETRIC_WEAK
import androidx.biometric.BiometricManager.Authenticators.DEVICE_CREDENTIAL
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import java.time.LocalDate
import uniffi.bpm_caddy_mobile.tr as rustTr

/** Un libellé de l'interface, lu dans `assets/strings.fr.toml`. */
fun T(key: String): String = rustTr(key)

/** Aujourd'hui, en ISO : la bibliothèque n'a pas d'horloge. */
fun today(): String = LocalDate.now().toString()

/**
 * Le Wi-Fi filtre les annonces diffusées tant qu'une application ne tient
 * pas ce verrou : il est pris le temps d'écouter les postes, puis rendu.
 */
fun <T> withMulticast(context: Context, block: () -> T): T {
    val wifi = context.applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
    val lock = wifi.createMulticastLock("bpm-caddy-postes").apply { setReferenceCounted(false) }
    lock.acquire()
    try {
        return block()
    } finally {
        lock.release()
    }
}

/** Tous les combien le téléphone échange, tant qu'il est à l'écran. */
private const val SYNC_EVERY_MS = 60_000L

class MainActivity : FragmentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val state: AppState by viewModels()
        setContent {
            CompanionApp(state = state, unlock = { unlock(state) })
        }
        // **Tant que l'application est à l'écran**, un échange à chaque
        // retour au premier plan puis toutes les minutes — la messagerie
        // reste à jour pendant qu'on la lit. Rien quand elle n'est pas à
        // l'écran : c'est la décision de l'officine, pas d'arrière-plan.
        lifecycleScope.launch {
            repeatOnLifecycle(Lifecycle.State.STARTED) {
                while (true) {
                    if (state.status?.inGroup == true) state.sync()
                    delay(SYNC_EVERY_MS)
                }
            }
        }
    }

    /** L'authentification de l'écran de verrouillage, puis la base. */
    private fun unlock(state: AppState) {
        val allowed = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R) {
            BIOMETRIC_STRONG or DEVICE_CREDENTIAL
        } else {
            BIOMETRIC_WEAK or DEVICE_CREDENTIAL
        }
        val info = BiometricPrompt.PromptInfo.Builder()
            .setTitle(T("mobile_unlock_title"))
            .setSubtitle(T("mobile_unlock_subtitle"))
            .setAllowedAuthenticators(allowed)
            .build()
        val prompt = BiometricPrompt(
            this,
            ContextCompat.getMainExecutor(this),
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    state.open()
                }

                override fun onAuthenticationError(code: Int, text: CharSequence) {
                    state.notice(
                        if (code == BiometricPrompt.ERROR_NO_DEVICE_CREDENTIAL) {
                            T("mobile_unlock_no_lock")
                        } else {
                            text.toString()
                        },
                    )
                }
            },
        )
        prompt.authenticate(info)
    }
}
