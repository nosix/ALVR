package io.github.alvr.android.app

import android.content.Context
import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import android.view.SurfaceHolder
import android.view.SurfaceView
import io.github.alvr.android.lib.AlvrClient
import kotlinx.coroutines.asCoroutineDispatcher
import java.util.concurrent.Executors

class MainActivity : AppCompatActivity() {

    companion object {
        private val TAG = MainActivity::class.simpleName
    }

    private val mAlvrClient = AlvrClient(
        Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    )

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        mAlvrClient.attachPreference(getPreferences(Context.MODE_PRIVATE))
        lifecycle.addObserver(mAlvrClient)
    }

    override fun onResume() {
        super.onResume()
        val surfaceHolder = findViewById<SurfaceView>(R.id.surface).holder
        surfaceHolder.addCallback(object : SurfaceHolder.Callback {
            override fun surfaceCreated(holder: SurfaceHolder) {
                val rect = holder.surfaceFrame
                Log.d(TAG, "surfaceCreated $rect")
                mAlvrClient.attachScreen(holder.surface, rect.width(), rect.height())
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
                mAlvrClient.detachSurface()
            }
        })
    }
}