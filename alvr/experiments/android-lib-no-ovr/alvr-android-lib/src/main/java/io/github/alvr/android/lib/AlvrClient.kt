package io.github.alvr.android.lib

import android.content.SharedPreferences
import android.util.Log
import android.view.Surface
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.lifecycleScope
import androidx.lifecycle.repeatOnLifecycle
import io.github.alvr.android.lib.AlvrPreferences.Companion.get
import io.github.alvr.android.lib.AlvrPreferences.Companion.set
import io.github.alvr.android.lib.event.ConnectionEvent
import io.github.alvr.android.lib.event.ConnectionSettings
import io.github.alvr.android.lib.gl.GlContext
import io.github.alvr.android.lib.gl.GlSurface
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlin.coroutines.CoroutineContext

class AlvrClient(
    context: CoroutineContext = Dispatchers.Main + GlContext()
) : DefaultLifecycleObserver {

    companion object {
        private val TAG = AlvrClient::class.simpleName
    }

    private val mCoroutineContext: CoroutineContext =
        if (context[GlContext.Key] == null) {
            context + GlContext()
        } else {
            context
        }

    private var mSharedPreferences: SharedPreferences? = null
    private var mDataProducer: DeviceDataProducer? = null

    private lateinit var mNativeApi: NativeApi
    private lateinit var mDecoder: Decoder
    private var mIsReady = false

    private val mSettingsChannel = Channel<ConnectionSettings>(1, BufferOverflow.DROP_OLDEST)
    private val mScreenChannel = Channel<Screen>(1, BufferOverflow.DROP_OLDEST)

    fun attachPreference(shardPref: SharedPreferences) {
        mSharedPreferences = shardPref
    }

    fun attachDeviceDataProducer(producer: DeviceDataProducer) {
        mDataProducer = producer
    }

    fun attachScreen(surface: Surface, width: Int, height: Int) {
        if (!mIsReady) {
            throw RuntimeException("The decoder is not ready.")
        }
        check(mScreenChannel.trySend(Screen(surface, width, height)).isSuccess) {
            "Screen could not be attached."
        }
    }

    fun detachSurface() {
        if (!mIsReady) {
            throw RuntimeException("The decoder is not ready.")
        }
        Log.i(TAG, "Stop the decoder")
        mDecoder.stop()
    }

    override fun onCreate(owner: LifecycleOwner) {
        val shardPref = requireNotNull(mSharedPreferences) {
            "Call the attachPreference method before onCreate."
        }

        val preferences = shardPref.get().also {
            Log.i(TAG, "load $it")
        }

        mNativeApi = NativeApi()
        if (mNativeApi.initPreferences(preferences)) {
            shardPref.set(preferences)
            Log.i(TAG, "save $preferences")
        }
        mDataProducer?.attach(mNativeApi)
        mNativeApi.setConnectionObserver(ConnectionObserver { event ->
            when (event) {
                is ConnectionEvent.ServerFound -> {
                    Log.i(TAG, "Server found at ${event.ipaddr}")
                }
                is ConnectionEvent.Connected -> {
                    Log.i(TAG, "Connected ${event.settings}")
                    check(mSettingsChannel.trySend(event.settings).isSuccess) {
                        "Settings could not be attached."
                    }
                }
                is ConnectionEvent.Error -> {
                    Log.e(TAG, event.toString())
                }
                else -> Log.i(TAG, event.toString())
            }
        })

        mDecoder = Decoder(
            mCoroutineContext,
            onInputBufferAvailable = mNativeApi::onInputBufferAvailable,
            onOutputBufferAvailable = mNativeApi::onOutputBufferAvailable,
            onRendered = mNativeApi::onRendered
        )

        owner.lifecycleScope.launch(mCoroutineContext) {
            owner.repeatOnLifecycle(Lifecycle.State.STARTED) {
                try {
                    val context = checkNotNull(coroutineContext[GlContext.Key]) {
                        "GlContext is not set to CoroutineContext."
                    }
                    while (isActive) {
                        val settings = mSettingsChannel.receive()
                        val screen = mScreenChannel.receive()
                        mDecoder.start(
                            settings.codec,
                            settings.realtime,
                            GlSurface(context, screen.surface),
                            screen.width,
                            screen.height
                        )
                    }
                } finally {
                    mDecoder.stop()
                }
            }
        }

        mIsReady = true
    }

    override fun onStart(owner: LifecycleOwner) {
        mNativeApi.onStart()
    }

    override fun onStop(owner: LifecycleOwner) {
        mNativeApi.onStop()
    }
}