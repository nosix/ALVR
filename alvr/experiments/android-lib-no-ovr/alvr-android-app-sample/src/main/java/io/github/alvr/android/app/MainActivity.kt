package io.github.alvr.android.app

import android.content.Context
import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import android.view.SurfaceHolder
import android.view.SurfaceView
import io.github.alvr.android.lib.AlvrPreferences.Companion.get
import io.github.alvr.android.lib.AlvrPreferences.Companion.set
import io.github.alvr.android.lib.ConnectionObserver
import io.github.alvr.android.lib.Decoder
import io.github.alvr.android.lib.NativeApi
import io.github.alvr.android.lib.VideoFormat

class MainActivity : AppCompatActivity() {

    companion object {
        private val TAG = MainActivity::class.simpleName
    }

    lateinit var nativeApi: NativeApi
    lateinit var decoder: Decoder

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val sharedPref = getPreferences(Context.MODE_PRIVATE)
        val preferences = sharedPref.get()
        Log.i(TAG, "load $preferences")

        nativeApi = NativeApi()
        if (nativeApi.initPreferences(preferences)) {
            sharedPref.set(preferences)
            Log.i(TAG, "save $preferences")
        }
        nativeApi.setConnectionObserver(ConnectionObserver { event ->
            // TODO implement this
            Log.i("Observe", event.toString())
        })
        nativeApi.onCreate()

        decoder = Decoder(
            onInputBufferAvailable = { inputBuffer ->
                nativeApi.notifyAvailableInputBuffer(inputBuffer)
            },
            onOutputBufferAvailable = { frameIndex ->
                nativeApi.notifyAvailableOutputBuffer(frameIndex)
            }
        )
    }

    override fun onStart() {
        super.onStart()
        nativeApi.onStart()
    }

    override fun onResume() {
        super.onResume()
        val surfaceHolder = findViewById<SurfaceView>(R.id.surface).holder
        surfaceHolder.addCallback(object : SurfaceHolder.Callback {
            override fun surfaceCreated(holder: SurfaceHolder) {
                val rect = holder.surfaceFrame
                Log.d(TAG, "surfaceCreated $rect")
                decoder.start(VideoFormat.H264, true, holder.surface)
            }

            override fun surfaceChanged(
                holder: SurfaceHolder,
                format: Int,
                width: Int,
                height: Int
            ) {
                val rect = holder.surfaceFrame
                Log.d(TAG, "surfaceChanged $rect")
            }

            override fun surfaceDestroyed(holder: SurfaceHolder) {
                Log.d(TAG, "surfaceDestroyed")
                decoder.stop()
            }
        })
    }

    override fun onStop() {
        super.onStop()
        nativeApi.onStop()
    }
}