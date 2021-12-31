package io.github.alvr.android.lib

import android.media.MediaCodec
import android.util.Log
import java.nio.ByteBuffer

class InputBuffer(
    @Suppress("MemberVisibilityCanBePrivate") // publish to native code
    val buffer: ByteBuffer,
    private val index: Int,
    private val codec: MediaCodec,
    private val frameMap: FrameMap
) {
    init {
        if (!buffer.isDirect) {
            throw IllegalStateException("InputBuffer must be direct.")
        }
    }

    @Suppress("unused") // publish to native code
    fun queueConfig() {
        val presentationTimeUs: Long = 0
        val flags: Int = MediaCodec.BUFFER_FLAG_CODEC_CONFIG
        codec.queueInputBuffer(index, 0, buffer.position(), presentationTimeUs, flags)
    }

    @Suppress("unused") // publish to native code
    fun queue(frameIndex: Long) {
        val presentationTimeUs: Long = System.nanoTime() / 1000
        val flags = 0
        codec.queueInputBuffer(index, 0, buffer.position(), presentationTimeUs, flags)
        frameMap.put(presentationTimeUs, frameIndex)
    }
}