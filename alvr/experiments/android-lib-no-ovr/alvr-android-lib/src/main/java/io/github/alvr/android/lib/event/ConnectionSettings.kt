package io.github.alvr.android.lib.event

import io.github.alvr.android.lib.gl.FfrParam
import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
data class ConnectionSettings(
    val fps: Float,
    val codec: AlvrCodec,
    val realtime: Boolean,
    @SerialName("dashboard_url")
    val dashboardUrl: String,
    @SerialName("ffr_param")
    val ffrParam: FfrParam?
)