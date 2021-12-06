package io.github.alvr.android.lib.event

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class ConnectionEvent {

    @Serializable
    @SerialName("Initial")
    object Initial : ConnectionEvent()

    @Serializable
    @SerialName("ServerFound")
    data class ServerFound(val ipaddr: String) : ConnectionEvent()

    @Serializable
    @SerialName("Connected")
    data class Connected(val settings: ConnectionSettings) : ConnectionEvent()

    @Serializable
    @SerialName("StreamStart")
    object StreamStart : ConnectionEvent()

    @Serializable
    @SerialName("ServerRestart")
    object ServerRestart : ConnectionEvent()

    @Serializable
    @SerialName("Error")
    data class Error(val error: ConnectionError) : ConnectionEvent()
}