package io.github.alvr.android.lib

import android.media.MediaCodec
import android.media.MediaCodecList
import android.media.MediaFormat
import android.util.Log
import io.github.alvr.android.lib.event.AlvrCodec
import io.github.alvr.android.lib.gl.GlSurface
import io.github.alvr.android.lib.gl.Renderer
import io.github.alvr.android.lib.gl.SurfaceHolder
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import java.nio.ByteBuffer
import kotlin.coroutines.CoroutineContext

class Decoder(
    context: CoroutineContext,
    private val onInputBufferAvailable: (InputBuffer) -> Unit,
    private val onOutputBufferAvailable: (Long) -> Unit
) {
    companion object {
        private val TAG = Decoder::class.simpleName
    }

    private val mScope = CoroutineScope(context)
    private val mFrameMap = FrameMap()

    private var mGlSurface: GlSurface? = null
    private var mFrameSurface: SurfaceHolder? = null
    private var mRenderer: Renderer? = null
    private var mCodec: MediaCodec? = null

    fun start(
        videoFormat: AlvrCodec,
        isRealTime: Boolean,
        surface: GlSurface,
        width: Int,
        height: Int
    ) {
        mScope.launch {
            stopInternal()
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
            mCodec = codec
            mGlSurface = surface
            mFrameSurface = frameSurface
            mRenderer = Renderer(surface, width, height)
            Log.i(TAG, "The decoder has started.")
        }
    }

    fun stop() {
        mScope.launch {
            stopInternal()
        }
    }

    private fun stopInternal() {
        mCodec?.run {
            mCodec = null
            stop()
            release()
            Log.i(TAG, "The codec has stopped.")
        }
        mGlSurface?.run {
            mGlSurface = null
            mFrameSurface?.let { surface ->
                mFrameSurface = null
                context.releaseSurface(surface)
                Log.i(TAG, "The frame surface has released.")
            }
            release()
            Log.i(TAG, "The EGLSurface has destroyed.")
        }
    }

    private val mMediaCodecCallback = object : MediaCodec.Callback() {
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
                // TODO reduce launch
                mScope.launch {
                    mFrameSurface?.let { frame ->
                        mRenderer?.render(frame)
                    }
                }
                this@Decoder.onOutputBufferAvailable(frameIndex)
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