package io.github.alvr.android.lib

import android.app.Activity
import android.opengl.EGL14
import android.util.Log
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.yield

/**
 * This class used from Unity.
 *
 * Example Unity Script:
 * <pre>
 * [DllImport("alvr_android")]
 * private static extern IntPtr GetInitContextEventFunc();
 *
 * private void Awake()
 * {
 *     using var unityPlayer = new AndroidJavaClass("com.unity3d.player.UnityPlayer");
 *     using var activity = unityPlayer.GetStatic<AndroidJavaObject>("currentActivity");
 *     _androidPluginInstance = new AndroidJavaObject("io.github.alvr.android.lib.UnityPlugin");
 *     GL.IssuePluginEvent(GetInitContextEventFunc(), 0);
 *     _androidPlugInInstance?.Call("onAwake");
 * }
 *
 * private void OnEnable()
 * {
 *     _androidPlugInInstance?.Call("onEnable");
 * }
 *
 * private void OnDisable()
 * {
 *     _androidPlugInInstance?.Call("onDisable");
 * }
 *
 * private void OnApplicationPause(bool pauseStatus)
 * {
 *     _androidPlugInInstance?.Call("onApplicationPause", pauseStatus);
 * }
 *
 * private void OnDestroy()
 * {
 *     _androidPlugInInstance?.Call("onDestroy");
 *     _androidPlugInInstance = null;
 * }
 * </pre>
 *
 * @see <a href="https://docs.unity3d.com/Manual/NativePluginInterface.html">
 *     Plug-in callbacks on the rendering thread</a>
 */
@Suppress("unused") // publish to Unity code
class UnityPlugin(activity: Activity) : LifecycleOwner {

    companion object {
        private val TAG = UnityPlugin::class.simpleName

        init {
            System.loadLibrary("alvr_android")
        }
    }

    private val mMainScope = CoroutineScope(Dispatchers.Main)
    private val mLifecycle = PluginLifecycle(this)
    private val mAlvrClient = AlvrClient()

    private var mEglSurface: ExternalEGLContext? = null

    init {
        attach()
        mAlvrClient.attachPreference(activity.getPreferences(Activity.MODE_PRIVATE))
    }

    private external fun attach()

    /**
     * This method callbacks on the Unity's rendering thread
     */
    @Suppress("unused") // publish to native code
    fun initContext() {
        Log.d(TAG, "[${Thread.currentThread().name}] initContext called")

        val unityContext = EGL14.eglGetCurrentContext()
        if (unityContext == EGL14.EGL_NO_CONTEXT) {
            throw IllegalStateException("Unity EGLContext is invalid")
        }

        mMainScope.launch {
            mEglSurface = ExternalEGLContext(unityContext)
        }
    }

    fun onAwake() {
        mMainScope.launch {
            mLifecycle.addObserver(mAlvrClient)
            mLifecycle.onCreate()
        }
    }

    fun onEnable() {
        mMainScope.launch {
            mLifecycle.onStart()
        }
    }

    fun onApplicationPause(pauseStatus: Boolean) {
        mMainScope.launch {
            if (pauseStatus) {
                mLifecycle.onPause()
            } else {
                mLifecycle.onResume()
            }
        }
    }

    fun onDisable() {
        mMainScope.launch {
            mLifecycle.onStop()
        }
    }

    fun onDestroy() {
        mMainScope.cancel()
        mEglSurface?.close()
        mLifecycle.onDestroy()
    }

    fun attachTexture(texturePtr: Int, width: Int, height: Int) {
        val eglSurface = checkNotNull(mEglSurface) { "EGLSurface is not initialized" }
        mMainScope.launch {
            val texture = eglSurface.createSurfaceTexture(texturePtr, width, height)
            var isFrameAvailable = false
            val surface = eglSurface.createSurface(texture) {
                isFrameAvailable = true
            }
            try {
                mAlvrClient.attachSurface(surface)
                while (true) {
                    // TODO use channel
                    if (!isFrameAvailable) {
                        delay(100)
                        continue
                    }
                    isFrameAvailable = false
                    eglSurface.updateTexImage(texture)
                    yield()
                }
            } finally {
                surface.release()
                texture.release()
            }
        }
    }

    fun detachTexture() {
        mMainScope.launch {
            mAlvrClient.detachSurface()
        }
    }

    override fun getLifecycle(): Lifecycle = mLifecycle

    private class PluginLifecycle(owner: LifecycleOwner) : Lifecycle() {
        private val mRegistry = LifecycleRegistry(owner)

        override fun getCurrentState(): State = mRegistry.currentState

        override fun addObserver(observer: LifecycleObserver) {
            mRegistry.addObserver(observer)
        }

        override fun removeObserver(observer: LifecycleObserver) {
            mRegistry.removeObserver(observer)
        }

        fun onCreate() {
            mRegistry.handleLifecycleEvent(Event.ON_CREATE)
        }

        fun onStart() {
            mRegistry.handleLifecycleEvent(Event.ON_START)
        }

        fun onResume() {
            mRegistry.handleLifecycleEvent(Event.ON_RESUME)
        }

        fun onPause() {
            mRegistry.handleLifecycleEvent(Event.ON_PAUSE)
        }

        fun onStop() {
            mRegistry.handleLifecycleEvent(Event.ON_STOP)
        }

        fun onDestroy() {
            mRegistry.handleLifecycleEvent(Event.ON_DESTROY)
        }
    }
}