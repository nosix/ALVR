package io.github.alvr.android.app

import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import io.github.alvr.android.lib.NativeApi

class MainActivity : AppCompatActivity() {

    lateinit var nativeApi: NativeApi

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        nativeApi = NativeApi()
        Log.d("MainActivity", nativeApi.stringFromJni())
        nativeApi.onCreate()
    }

    override fun onResume() {
        super.onResume()
        nativeApi.onResume()
    }
}