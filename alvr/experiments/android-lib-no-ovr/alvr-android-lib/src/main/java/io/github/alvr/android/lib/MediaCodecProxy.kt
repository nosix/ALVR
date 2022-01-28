package io.github.alvr.android.lib

import android.media.MediaCodec
import android.util.Log
import java.nio.ByteBuffer

class MediaCodecProxy {

    private companion object {
        private val TAG = MediaCodecProxy::class.simpleName
    }

    private var mCodec: MediaCodec? = null

    private val isActive: Boolean
        get() = mCodec != null

    fun start(codec: MediaCodec) {
        mCodec = codec
        codec.start()
    }

    fun stop() {
        mCodec?.let { codec ->
            mCodec = null
            codec.stop()
            codec.release()
        }
    }

    fun getInputBuffer(codec: MediaCodec, index: Int, frameMap: FrameMap): InputBuffer? {
        if (!isActive) {
            Log.w(TAG, "MediaCodec is not active.")
            return null
        }
        if (mCodec != codec) {
            Log.e(TAG, "Invalid MediaCodec")
            return null
        }
        val buffer: ByteBuffer = codec.getInputBuffer(index) ?: return null
        // TODO recycle InputBuffer object
        return InputBuffer(buffer, index, this, frameMap)
    }

    fun releaseOutputBuffer(codec: MediaCodec, index: Int): Boolean {
        if (!isActive) {
            Log.w(TAG, "MediaCodec is not active.")
            return false
        }
        if (mCodec != codec) {
            Log.e(TAG, "Invalid MediaCodec")
            return false
        }
        codec.releaseOutputBuffer(index, true)
        return true
    }

    fun queueInputBuffer(index: Int, buffer: ByteBuffer, presentationTimeUs: Long, flags: Int) {
        mCodec?.queueInputBuffer(index, 0, buffer.position(), presentationTimeUs, flags)
    }
}