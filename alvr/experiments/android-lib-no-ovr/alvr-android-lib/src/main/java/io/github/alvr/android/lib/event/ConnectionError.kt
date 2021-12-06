package io.github.alvr.android.lib.event

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class ConnectionError {

    @Serializable
    @SerialName("NetworkUnreachable")
    object NetworkUnreachable : ConnectionError()

    @Serializable
    @SerialName("ClientUntrusted")
    object ClientUntrusted : ConnectionError()

    @Serializable
    @SerialName("IncompatibleVersions")
    object IncompatibleVersions : ConnectionError()

    @Serializable
    @SerialName("TimeoutSetUpStream")
    object TimeoutSetUpStream : ConnectionError()

    @Serializable
    @SerialName("ServerDisconnected")
    data class ServerDisconnected(val cause: String) : ConnectionError()

    @Serializable
    @SerialName("SystemError")
    data class SystemError(val cause: String) : ConnectionError()
}