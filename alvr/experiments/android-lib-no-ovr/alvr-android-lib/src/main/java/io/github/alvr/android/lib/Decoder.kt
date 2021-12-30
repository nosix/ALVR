package io.github.alvr.android.lib

import android.media.MediaCodec
import android.media.MediaCodecList
import android.media.MediaFormat
import android.util.Log
import io.github.alvr.android.lib.event.ConnectionSettings
import io.github.alvr.android.lib.gl.GlContext
import io.github.alvr.android.lib.gl.GlSurface
import io.github.alvr.android.lib.gl.PASS_THROUGH_FRAGMENT_SHADER
import io.github.alvr.android.lib.gl.Renderer
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.channels.BufferOverflow
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
    private val mUpdatedSignal = Channel<Long>() // FIXME Do not create a Long instance
    private val mPauseSignal = Channel<Unit>(1)
    private val mSettingsChannel = Channel<ConnectionSettings>(1, BufferOverflow.DROP_OLDEST)
    private var mDecodeJob: Job? = null
    private var isActive: Boolean = false

    fun start(settings: ConnectionSettings, context: GlContext, screen: Screen) {
        mScope.launch {
            mDecodeJob?.cancelAndJoin()
            mDecodeJob = launch {
                mSettingsChannel.send(settings)
                decodeStream(context, screen)
            }
        }
    }

    fun pause() {
        mPauseSignal.trySend(Unit)
        mUpdatedSignal.trySend(0L) // Resume suspended receive
    }

    fun restart(settings: ConnectionSettings) {
        mSettingsChannel.trySend(settings)
    }

    fun stop() {
        mScope.launch {
            mDecodeJob?.let { job ->
                mDecodeJob = null
                job.cancelAndJoin()
            }
        }
    }

    private suspend fun decodeStream(context: GlContext, screen: Screen) {
        val glSurface = GlSurface(context, screen.surface)
        try {
            while (coroutineContext.isActive) {
                val settings = mSettingsChannel.receive()
                renderLoop(settings, glSurface, screen.width, screen.height)
            }
        } finally {
            glSurface.release()
            screen.onDetached()
        }
    }

    private suspend fun renderLoop(
        settings: ConnectionSettings,
        surface: GlSurface,
        width: Int,
        height: Int
    ) {
        val videoFormat = settings.codec.mime

        val frameSurface = surface.context.createSurface(width, height)
        val format = MediaFormat.createVideoFormat(videoFormat, width, height).apply {
            setString("KEY_MIME", videoFormat)
            setInteger("vendor.qti-ext-dec-low-latency.enable", 1) //Qualcomm low latency mode
            setInteger(MediaFormat.KEY_OPERATING_RATE, Short.MAX_VALUE.toInt())
            setInteger(MediaFormat.KEY_PRIORITY, if (settings.realtime) 0 else 1)
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
        isActive = true
        Log.i(TAG, "The codec has started.")

        try {
            val fragmentShaderCode =
                settings.ffrParam?.getFragmentShader()
                    ?: PASS_THROUGH_FRAGMENT_SHADER
            val renderer = Renderer(surface, width, height, fragmentShaderCode)
            while (coroutineContext.isActive) {
                val frameIndex = mUpdatedSignal.receive()
                if (isPaused()) {
                    return
                }
                renderer.render(frameSurface)
                onRendered(frameIndex)
            }
        } finally {
            isActive = false
            codec.stop()
            codec.release()
            surface.context.releaseSurface(frameSurface)
            Log.i(TAG, "The codec has stopped.")
        }
    }

    private fun isPaused(): Boolean {
        if (mPauseSignal.tryReceive().isSuccess) {
            while (mUpdatedSignal.tryReceive().isSuccess) {
                // Drop all signals
            }
            return true
        }
        return false
    }

    private val mMediaCodecCallback = object : MediaCodec.Callback() {

        private val mFrameMap = FrameMap()

        override fun onInputBufferAvailable(
            codec: MediaCodec, index: Int
        ) {
            if (!isActive) return
            val buffer: ByteBuffer = codec.getInputBuffer(index) ?: return
            // TODO recycle InputBuffer object
            val wrapper = InputBuffer(buffer, index, codec, mFrameMap)
            this@Decoder.onInputBufferAvailable(wrapper)
        }

        override fun onOutputBufferAvailable(
            codec: MediaCodec, index: Int, info: MediaCodec.BufferInfo
        ) {
            if (!isActive) return
            codec.releaseOutputBuffer(index, true)
            val frameIndex = mFrameMap.remove(info.presentationTimeUs)
            if (frameIndex != 0L) {
                this@Decoder.onOutputBufferAvailable(frameIndex)
                mUpdatedSignal.trySend(frameIndex)
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