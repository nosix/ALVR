package io.github.alvr.android.lib

import android.media.MediaCodec
import java.nio.ByteBuffer

class InputBuffer(
    @Suppress("MemberVisibilityCanBePrivate") // publish to native code
    val buffer: ByteBuffer,
    private val index: Int,
    private val codec: MediaCodec
) {
    fun queueConfig() {
        val presentationTimeUs: Long = 0
        val flags: Int = MediaCodec.BUFFER_FLAG_CODEC_CONFIG
        codec.queueInputBuffer(index, 0, buffer.position(), presentationTimeUs, flags)
    }

    fun queue() {
        val presentationTimeUs: Long = System.nanoTime() / 1000
        val flags: Int = 0
        codec.queueInputBuffer(index, 0, buffer.position(), presentationTimeUs, flags)
    }
}