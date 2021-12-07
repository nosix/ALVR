package io.github.alvr.android.lib.event

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

@Serializable
sealed class AlvrCodec(val mime: String) {

    @Serializable
    @SerialName("H264")
    object H264 : AlvrCodec("video/avc")

    @Serializable
    @SerialName("H265")
    object H265 : AlvrCodec("video/hevc")

    @Serializable
    @SerialName("Unknown")
    object Unknown : AlvrCodec("video/unknown")
}