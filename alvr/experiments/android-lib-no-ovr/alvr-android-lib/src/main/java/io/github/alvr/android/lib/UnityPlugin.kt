package io.github.alvr.android.lib

import android.util.Log

/**
 * This class used from Unity.
 *
 * Example Unity Script:
 * <pre>
 * [DllImport("alvr_android")]
 * private static extern IntPtr GetInitContextEventFunc();
 *
 * private void Start()
 * {
 *     _androidPluginInstance = new AndroidJavaObject("io.github.alvr.android.lib.UnityPlugin");
 *     GL.IssuePluginEvent(GetInitContextEventFunc(), 0);
 * }
 * </pre>
 *
 * @see https://docs.unity3d.com/Manual/NativePluginInterface.html
 */
@Suppress("unused") // publish to Unity code
class UnityPlugin {

    companion object {
        private val TAG = UnityPlugin::class.simpleName

        init {
            System.loadLibrary("alvr_android")
        }
    }

    init {
        attach()
    }

    /**
     * This method callbacks on the Unity's rendering thread
     */
    @Suppress("unused") // publish to native code
    fun initContext() {
        Log.d(TAG, "[${Thread.currentThread().name}] initContext called")
    }

    private external fun attach()
}