package io.github.alvr.android.app

import android.content.Context
import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import android.view.KeyEvent
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.widget.Toast
import io.github.alvr.android.lib.AlvrClient
import io.github.alvr.android.lib.ClientEventObserver
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

    private val mDeviceAdapter = DeviceAdapterImpl(
        DeviceSettings(
            name = "Android ALVR",
            recommendedEyeWidth = 1920,
            recommendedEyeHeight = 1080,
            availableRefreshRates = floatArrayOf(60.0f),
            preferredRefreshRate = 60.0f
        )
    )

    private val mEventObserver = object : ClientEventObserver {
        override fun onEventOccurred(eventJson: String) {
            Toast.makeText(this@MainActivity, eventJson, Toast.LENGTH_LONG).show()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        mAlvrClient.attachPreference(getPreferences(Context.MODE_PRIVATE))
        mAlvrClient.attachDeviceAdapter(mDeviceAdapter)
        mAlvrClient.setEventObserver(mEventObserver)
        lifecycle.addObserver(mAlvrClient)
        lifecycle.addObserver(mDeviceAdapter)
    }

    override fun dispatchKeyEvent(event: KeyEvent): Boolean {
        when (event.keyCode) {
            KeyEvent.KEYCODE_W -> mDeviceAdapter.moveToUp = event.isOn
            KeyEvent.KEYCODE_A -> mDeviceAdapter.moveToLeft = event.isOn
            KeyEvent.KEYCODE_S -> mDeviceAdapter.moveToDown = event.isOn
            KeyEvent.KEYCODE_D -> mDeviceAdapter.moveToRight = event.isOn
            KeyEvent.KEYCODE_R -> mDeviceAdapter.moveToBack = event.isOn
            KeyEvent.KEYCODE_F -> mDeviceAdapter.moveToForward = event.isOn
            KeyEvent.KEYCODE_I -> mDeviceAdapter.rotateUp = event.isOn
            KeyEvent.KEYCODE_J -> mDeviceAdapter.rotateLeft = event.isOn
            KeyEvent.KEYCODE_K -> mDeviceAdapter.rotateDown = event.isOn
            KeyEvent.KEYCODE_L -> mDeviceAdapter.rotateRight = event.isOn
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
                mAlvrClient.attachScreen(holder.surface, rect.width(), rect.height()) {}
            }

            override fun surfaceDestroyed(holder: SurfaceHolder) {
                Log.d(TAG, "surfaceDestroyed")
                mAlvrClient.detachScreen()
            }
        })
    }
}