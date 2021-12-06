package io.github.alvr.android.lib

import android.media.MediaCodec
import android.media.MediaCodecList
import android.media.MediaFormat
import android.util.Log
import android.view.Surface
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import java.nio.ByteBuffer

class Decoder(
    dispatcher: CoroutineDispatcher = Dispatchers.Default,
    private val onInputBufferAvailable: (InputBuffer) -> Unit,
    private val onOutputBufferAvailable: (Long) -> Unit
) {
    companion object {
        private val TAG = Decoder::class.simpleName
    }

    private val mScope = CoroutineScope(dispatcher)

    private var mCodec: MediaCodec? = null

    fun start(videoFormat: VideoFormat, isRealTime: Boolean, surface: Surface) {
        mScope.launch {
            stopInternal()
            val format = MediaFormat.createVideoFormat(videoFormat.mime, 512, 1024).apply {
                setString("KEY_MIME", videoFormat.mime)
                setInteger("vendor.qti-ext-dec-low-latency.enable", 1) //Qualcomm low latency mode
                setInteger(MediaFormat.KEY_OPERATING_RATE, Short.MAX_VALUE.toInt())
                setInteger(MediaFormat.KEY_PRIORITY, if (isRealTime) 0 else 1)
//            setByteBuffer("csd-0", ByteBuffer.wrap(nal.buf, 0, nal.buf.length))
            }
            val codecs = MediaCodecList(MediaCodecList.REGULAR_CODECS)
            @Suppress("BlockingMethodInNonBlockingContext")
            val codec = MediaCodec.createByCodecName(codecs.findDecoderForFormat(format)).apply {
                setVideoScalingMode(MediaCodec.VIDEO_SCALING_MODE_SCALE_TO_FIT)
                setCallback(mMediaCodecCallback)
                configure(format, surface, null, 0)
            }
            codec.start()
            mCodec = codec
        }
    }

    fun stop() {
        mScope.launch {
            stopInternal()
        }
    }

    private fun stopInternal() {
        mCodec?.run {
            stop()
            release()
            mCodec = null
        }
    }

    private val mMediaCodecCallback = object : MediaCodec.Callback() {
        override fun onInputBufferAvailable(
            codec: MediaCodec, index: Int
        ) {
            val buffer: ByteBuffer = codec.getInputBuffer(index) ?: return
            val wrapper = InputBuffer(buffer, index, codec)
            this@Decoder.onInputBufferAvailable(wrapper)
        }

        override fun onOutputBufferAvailable(
            codec: MediaCodec, index: Int, info: MediaCodec.BufferInfo
        ) {
            codec.releaseOutputBuffer(index, true)
            this@Decoder.onOutputBufferAvailable(info.presentationTimeUs)
        }

        override fun onOutputFormatChanged(codec: MediaCodec, format: MediaFormat) {
            Log.i(TAG, "onOutputFormatChanged $format");
        }

        override fun onError(codec: MediaCodec, e: MediaCodec.CodecException) {
            Log.e(TAG, e.toString())
        }
    }
}