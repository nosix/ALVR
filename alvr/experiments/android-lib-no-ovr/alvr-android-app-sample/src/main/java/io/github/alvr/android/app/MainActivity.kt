package io.github.alvr.android.app

import android.content.Context
import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import android.view.KeyEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import io.github.alvr.android.lib.AlvrClient
import io.github.alvr.android.lib.DeviceSettings
import kotlinx.coroutines.asCoroutineDispatcher
import java.util.concurrent.Executors

class MainActivity : AppCompatActivity() {

    companion object {
        private val TAG = MainActivity::class.simpleName
    }

    private val mAlvrClient = AlvrClient(
        Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    )

    private val mDataProducer = DeviceDataProducerImpl(
        DeviceSettings(
            name = "Android ALVR",
            recommendedEyeWidth = 1920,
            recommendedEyeHeight = 1080,
            availableRefreshRates = floatArrayOf(60.0f),
            preferredRefreshRate = 60.0f
        )
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        mAlvrClient.attachPreference(getPreferences(Context.MODE_PRIVATE))
        mAlvrClient.attachDeviceDataProducer(mDataProducer)
        lifecycle.addObserver(mAlvrClient)
        lifecycle.addObserver(mDataProducer)
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        when (event.keyCode) {
            KeyEvent.KEYCODE_W -> mDataProducer.moveToUp = event.isOn
            KeyEvent.KEYCODE_A -> mDataProducer.moveToLeft = event.isOn
            KeyEvent.KEYCODE_S -> mDataProducer.moveToDown = event.isOn
            KeyEvent.KEYCODE_D -> mDataProducer.moveToRight = event.isOn
            KeyEvent.KEYCODE_R -> mDataProducer.moveToBack = event.isOn
            KeyEvent.KEYCODE_F -> mDataProducer.moveToForward = event.isOn
            KeyEvent.KEYCODE_I -> mDataProducer.rotateUp = event.isOn
            KeyEvent.KEYCODE_J -> mDataProducer.rotateLeft = event.isOn
            KeyEvent.KEYCODE_K -> mDataProducer.rotateDown = event.isOn
            KeyEvent.KEYCODE_L -> mDataProducer.rotateRight = event.isOn
        }
        return super.dispatchKeyEvent(event)
    }

    private val KeyEvent.isOn: Boolean
        get() = this.action == KeyEvent.ACTION_DOWN

    override fun onResume() {
        super.onResume()
        val surfaceHolder = findViewById<SurfaceView>(R.id.surface).holder
        surfaceHolder.addCallback(object : SurfaceHolder.Callback {
            override fun surfaceCreated(holder: SurfaceHolder) {
                val rect = holder.surfaceFrame
                Log.d(TAG, "surfaceCreated $rect")
                mAlvrClient.attachScreen(holder.surface, rect.width(), rect.height()) {}
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
                mAlvrClient.detachScreen()
            }
        })
    }
}