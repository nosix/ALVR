package io.github.alvr.android.app

import android.content.Context
import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import io.github.alvr.android.lib.AlvrPreferences.Companion.get
import io.github.alvr.android.lib.AlvrPreferences.Companion.set
import io.github.alvr.android.lib.NativeApi

class MainActivity : AppCompatActivity() {

    companion object {
        private val TAG = MainActivity::class.simpleName
    }

    lateinit var nativeApi: NativeApi

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
        nativeApi.onCreate()
    }

    override fun onStart() {
        super.onStart()
        nativeApi.onStart()
    }

    override fun onStop() {
        super.onStop()
        nativeApi.onStop()
    }
}