package io.github.alvr.android.lib

import android.content.SharedPreferences

/**
 * Preferences shared with native code.
 *
 * The properties are 'var' because the value is changed by native code.
 */
data class AlvrPreferences(
    var hostname: String,
    var certificate_pem: String,
    var key_pem: String
) {
    companion object {
        private const val KEY_HOSTNAME = "hostname"
        private const val KEY_CERTIFICATE_PEM = "certificate_pem"
        private const val KEY_KEY_PEM = "key_pem"

        fun SharedPreferences.set(preferences: AlvrPreferences) {
            with(edit()) {
                putString(KEY_HOSTNAME, preferences.hostname)
                putString(KEY_CERTIFICATE_PEM, preferences.certificate_pem)
                putString(KEY_KEY_PEM, preferences.key_pem)
                apply()
            }
        }

        fun SharedPreferences.get(): AlvrPreferences = AlvrPreferences(
            getString(KEY_HOSTNAME, null) ?: "",
            getString(KEY_CERTIFICATE_PEM, null) ?: "",
            getString(KEY_KEY_PEM, null) ?: ""
        )
    }
}