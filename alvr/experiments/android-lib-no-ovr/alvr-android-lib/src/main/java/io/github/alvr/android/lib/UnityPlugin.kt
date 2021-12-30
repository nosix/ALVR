package io.github.alvr.android.lib

import android.app.Activity
import android.opengl.EGL14
import android.util.Log
import android.view.Surface
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleObserver
import androidx.lifecycle.LifecycleOwner
import androidx.lifecycle.LifecycleRegistry
import io.github.alvr.android.lib.gl.GlContext
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.cancel
import kotlinx.coroutines.cancelAndJoin
import kotlinx.coroutines.channels.BufferOverflow
import kotlinx.coroutines.channels.Channel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.filterNotNull
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.concurrent.Executors
import kotlin.coroutines.coroutineContext

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

    private val mScope = CoroutineScope(Dispatchers.Main)
    private val mGlContext = MutableStateFlow<GlContext?>(null)
    private val mLifecycle = PluginLifecycle(this)

    private val mAlvrClient = AlvrClient(
        Executors.newSingleThreadExecutor().asCoroutineDispatcher()
    )

    private class Texture(val textureId: Int, val width: Int, val height: Int)

    private var mTexture: Texture? = null
    private var mCopyJob: Job? = null

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

        mScope.launch {
            mGlContext.value = GlContext(unityContext)
        }
    }

    private fun CoroutineScope.launchWithGlContext(action: suspend CoroutineScope.() -> Unit): Job {
        return launch {
            // Wait until GlContext is ready
            withContext(mGlContext.filterNotNull().first(), action)
        }
    }

    fun onAwake() {
        mScope.launchWithGlContext {
            mLifecycle.addObserver(mAlvrClient)
            mLifecycle.onCreate()
        }
    }

    fun onApplicationPause(pauseStatus: Boolean) {
        mScope.launchWithGlContext {
            if (pauseStatus) {
                mLifecycle.onStop()
                mCopyJob?.let { job ->
                    mCopyJob = null
                    job.cancelAndJoin()
                }
            } else {
                mLifecycle.onStart()
                mTexture?.let { texture ->
                    if (mCopyJob == null) {
                        mCopyJob = launch { copyTexture(texture) }
                    }
                }
            }
        }
    }

    fun onDestroy() {
        mScope.cancel()
        mLifecycle.onDestroy()
        mGlContext.value?.close()
    }

    fun attachTexture(textureId: Int, width: Int, height: Int) {
        mScope.launchWithGlContext {
            val texture = Texture(textureId, width, height)
            mTexture = texture
            mCopyJob = launch { copyTexture(texture) }
        }
    }

    private suspend fun copyTexture(texture: Texture) {
        val context = checkNotNull(coroutineContext[GlContext.Key])
        val holder = context.createSurfaceTexture(texture.textureId, texture.width, texture.height)
        val frameAvailable = Channel<Unit>(1, BufferOverflow.DROP_OLDEST)
        holder.surfaceTexture.setOnFrameAvailableListener {
            frameAvailable.trySend(Unit)
        }
        val surface = Surface(holder.surfaceTexture)
        try {
            mAlvrClient.attachScreen(surface, holder.width, holder.height) {
                surface.release()
                context.releaseSurfaceTexture(holder)
            }
            while (coroutineContext.isActive) {
                frameAvailable.receive()
                context.withMakeCurrent {
                    holder.updateTexImage()
                }
            }
        } finally {
            mAlvrClient.detachScreen()
        }
    }

    fun detachTexture() {
        mScope.launchWithGlContext {
            mTexture = null
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