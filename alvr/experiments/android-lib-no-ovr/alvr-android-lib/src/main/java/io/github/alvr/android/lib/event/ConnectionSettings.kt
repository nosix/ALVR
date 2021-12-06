package io.github.alvr.android.lib.event

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class ConnectionSettings(
    val fps: Float,
    val codec: AlvrCodec,
    val realtime: Boolean,
    @SerialName("dark_mode")
    val darkMode: Boolean,
    @SerialName("dashboard_url")
    val dashboardUrl: String,
)