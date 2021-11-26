package io.github.alvr.android.lib

class NativeApi {

    external fun stringFromJni(): String

    companion object {
        init {
            System.loadLibrary("alvr_android")
        }
    }
}