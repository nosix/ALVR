package io.github.alvr.android.lib

import android.media.MediaCodec
import android.media.MediaCodecList
import android.media.MediaFormat
import android.util.Log
import io.github.alvr.android.lib.event.AlvrCodec
import io.github.alvr.android.lib.gl.GlSurface
import io.github.alvr.android.lib.gl.Renderer
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import java.nio.ByteBuffer
import kotlin.coroutines.CoroutineContext
import kotlin.coroutines.coroutineContext

class Decoder(
    context: CoroutineContext,
    private val onInputBufferAvailable: (InputBuffer) -> Unit,
    private val onOutputBufferAvailable: (Long) -> Unit,
    private val onRendered: (Long) -> Unit
) {
    companion object {
        private val TAG = Decoder::class.simpleName
    }

    private val mScope = CoroutineScope(context)
    private val mUpdatedSignalChannel = Channel<Long>() // FIXME Do not create a Long instance
    private var mDecodeJob: Job? = null

    fun start(
        videoFormat: AlvrCodec,
        isRealTime: Boolean,
        surface: GlSurface,
        width: Int,
        height: Int
    ) {
        mScope.launch {
            mDecodeJob?.cancelAndJoin()
            mDecodeJob = launch {
                decodeStream(videoFormat, isRealTime, surface, width, height)
            }
        }
    }

    fun stop() {
        mScope.launch {
            mDecodeJob?.cancelAndJoin()
        }
    }

    private suspend fun decodeStream(
        videoFormat: AlvrCodec,
        isRealTime: Boolean,
        surface: GlSurface,
        width: Int,
        height: Int
    ) {
        val frameSurface = surface.context.createSurface(width, height)
        val format = MediaFormat.createVideoFormat(videoFormat.mime, width, height).apply {
            setString("KEY_MIME", videoFormat.mime)
            setInteger("vendor.qti-ext-dec-low-latency.enable", 1) //Qualcomm low latency mode
            setInteger(MediaFormat.KEY_OPERATING_RATE, Short.MAX_VALUE.toInt())
            setInteger(MediaFormat.KEY_PRIORITY, if (isRealTime) 0 else 1)
            //setByteBuffer("csd-0", ByteBuffer.wrap(sps_nal.buf, 0, sps_nal.buf.length))
        }
        val codecs = MediaCodecList(MediaCodecList.REGULAR_CODECS)

        @Suppress("BlockingMethodInNonBlockingContext")
        val codec = MediaCodec.createByCodecName(codecs.findDecoderForFormat(format)).apply {
            setVideoScalingMode(MediaCodec.VIDEO_SCALING_MODE_SCALE_TO_FIT)
            setCallback(mMediaCodecCallback)
            configure(format, frameSurface.surface, null, 0)
        }
        codec.start()
        Log.i(TAG, "The decoder has started.")

        try {
            val renderer = Renderer(surface, width, height)
            while (coroutineContext.isActive) {
                val frameIndex = mUpdatedSignalChannel.receive()
                renderer.render(frameSurface)
                onRendered(frameIndex)
            }
        } finally {
            codec.stop()
            codec.release()
            surface.context.releaseSurface(frameSurface)
            surface.release()
            Log.i(TAG, "The codec has stopped.")
        }
    }

    private val mMediaCodecCallback = object : MediaCodec.Callback() {

        private val mFrameMap = FrameMap()

        override fun onInputBufferAvailable(
            codec: MediaCodec, index: Int
        ) {
            val buffer: ByteBuffer = codec.getInputBuffer(index) ?: return
            // TODO recycle InputBuffer object
            val wrapper = InputBuffer(buffer, index, codec, mFrameMap)
            this@Decoder.onInputBufferAvailable(wrapper)
        }

        override fun onOutputBufferAvailable(
            codec: MediaCodec, index: Int, info: MediaCodec.BufferInfo
        ) {
            codec.releaseOutputBuffer(index, true)
            val frameIndex = mFrameMap.remove(info.presentationTimeUs)
            if (frameIndex != 0L) {
                this@Decoder.onOutputBufferAvailable(frameIndex)
                mUpdatedSignalChannel.trySend(frameIndex)
            } else {
                Log.w(TAG, "The frameIndex corresponding to presentationTimeUs was not found.")
            }
        }

        override fun onOutputFormatChanged(codec: MediaCodec, format: MediaFormat) {
            Log.i(TAG, "onOutputFormatChanged $format")
        }

        override fun onError(codec: MediaCodec, e: MediaCodec.CodecException) {
            Log.e(TAG, e.toString())
        }
    }
}