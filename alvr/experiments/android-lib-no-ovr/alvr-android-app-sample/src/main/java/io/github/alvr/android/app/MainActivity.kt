package io.github.alvr.android.app

import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import android.util.Log
import io.github.alvr.android.lib.NativeApi

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        Log.d("MainActivity", NativeApi().stringFromJni())
    }
}