package io.github.alvr.android.lib

import android.content.SharedPreferences
import android.util.Log
import android.view.Surface
import androidx.lifecycle.DefaultLifecycleObserver
import androidx.lifecycle.LifecycleOwner
import io.github.alvr.android.lib.AlvrPreferences.Companion.get
import io.github.alvr.android.lib.AlvrPreferences.Companion.set

class AlvrClient : DefaultLifecycleObserver {

    companion object {
        private val TAG = AlvrClient::class.simpleName
    }

    private var mSharedPreferences: SharedPreferences? = null

    private lateinit var mNativeApi: NativeApi
    private lateinit var mDecoder: Decoder
    private var mIsReady = false

    fun attachPreference(shardPref: SharedPreferences) {
        mSharedPreferences = shardPref
    }

    fun attachSurface(surface: Surface) {
        if (!mIsReady) {
            throw RuntimeException("Decoder is not ready.")
        }
        mDecoder.start(VideoFormat.H264, true, surface)
    }

    fun detachSurface() {
        if (!mIsReady) {
            throw RuntimeException("Decoder is not ready.")
        }
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
            // TODO implement this
            Log.i("Observe", event.toString())
        })
        mNativeApi.onCreate()

        mDecoder = Decoder(
            onInputBufferAvailable = { inputBuffer ->
                mNativeApi.notifyAvailableInputBuffer(inputBuffer)
            },
            onOutputBufferAvailable = { frameIndex ->
                mNativeApi.notifyAvailableOutputBuffer(frameIndex)
            }
        )

        mIsReady = true
    }

    override fun onStart(owner: LifecycleOwner) {
        mNativeApi.onStart()
    }

    override fun onStop(owner: LifecycleOwner) {
        mNativeApi.onStop()
    }
}