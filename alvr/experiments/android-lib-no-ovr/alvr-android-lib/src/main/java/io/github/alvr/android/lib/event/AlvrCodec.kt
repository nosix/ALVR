package io.github.alvr.android.lib.event

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class AlvrCodec {

    @Serializable
    @SerialName("H264")
    object H264 : AlvrCodec()

    @Serializable
    @SerialName("H265")
    object H265 : AlvrCodec()

    @Serializable
    @SerialName("Unknown")
    object Unknown : AlvrCodec()
}