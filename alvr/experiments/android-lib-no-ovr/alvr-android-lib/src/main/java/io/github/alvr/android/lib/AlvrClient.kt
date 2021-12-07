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
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class AlvrClient(
    private val dispatcher: CoroutineDispatcher = Dispatchers.Default
) : DefaultLifecycleObserver {

    companion object {
        private val TAG = AlvrClient::class.simpleName
    }

    private var mSharedPreferences: SharedPreferences? = null

    private lateinit var mNativeApi: NativeApi
    private lateinit var mDecoder: Decoder
    private var mIsReady = false

    private val mSettingsChannel = Channel<ConnectionSettings>(1, BufferOverflow.DROP_OLDEST)
    private val mSurfaceChannel = Channel<Surface>(1, BufferOverflow.DROP_OLDEST)

    fun attachPreference(shardPref: SharedPreferences) {
        mSharedPreferences = shardPref
    }

    fun attachSurface(surface: Surface) {
        if (!mIsReady) {
            throw RuntimeException("The decoder is not ready.")
        }
        check(mSurfaceChannel.trySend(surface).isSuccess) {
            "Surface could not be attached."
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
            "Call the loadPreference method with onCreate."
        }

        val preferences = shardPref.get().also {
            Log.i(TAG, "load $it")
        }

        mNativeApi = NativeApi()
        if (mNativeApi.initPreferences(preferences)) {
            shardPref.set(preferences)
            Log.i(TAG, "save $preferences")
        }
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
        mNativeApi.onCreate()

        mDecoder = Decoder(
            dispatcher,
            onInputBufferAvailable = { inputBuffer ->
                mNativeApi.notifyAvailableInputBuffer(inputBuffer)
            },
            onOutputBufferAvailable = { frameIndex ->
                mNativeApi.notifyAvailableOutputBuffer(frameIndex)
            }
        )

        owner.lifecycleScope.launch(dispatcher) {
            owner.repeatOnLifecycle(Lifecycle.State.STARTED) {
                try {
                    while (isActive) {
                        val settings = mSettingsChannel.receive()
                        val surface = mSurfaceChannel.receive()
                        mDecoder.start(settings.codec, settings.realtime, surface)
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