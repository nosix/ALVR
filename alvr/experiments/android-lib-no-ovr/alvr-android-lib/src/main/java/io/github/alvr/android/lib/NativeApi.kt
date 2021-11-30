package io.github.alvr.android.lib

class NativeApi {

    external fun stringFromJni(): String
    external fun onCreate()
    external fun onResume()

    companion object {
        init {
            System.loadLibrary("alvr_android")
        }
    }
}