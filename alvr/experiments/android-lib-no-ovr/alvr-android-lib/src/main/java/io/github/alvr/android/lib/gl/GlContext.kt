package io.github.alvr.android.lib.gl

import android.graphics.SurfaceTexture
import android.opengl.EGL14
import android.opengl.EGLConfig
import android.opengl.EGLContext
import android.opengl.EGLDisplay
import android.opengl.EGLSurface
import android.opengl.GLES11Ext
import android.opengl.GLES32
import android.util.Log
import android.view.Surface
import kotlin.coroutines.AbstractCoroutineContextElement
import kotlin.coroutines.CoroutineContext

class GlContext(
    shareContext: EGLContext = EGL14.EGL_NO_CONTEXT
) : AutoCloseable, AbstractCoroutineContextElement(Key) {

    companion object {
        val Key = object : CoroutineContext.Key<GlContext> {}

        private val TAG = GlContext::class.simpleName
    }

    val display: EGLDisplay
    val context: EGLContext
    val config: EGLConfig

    var isClosed: Boolean = false
        private set

    init {
        val eglDisplay = EGL14.eglGetDisplay(EGL14.EGL_DEFAULT_DISPLAY)
        if (eglDisplay == EGL14.EGL_NO_DISPLAY) {
            throw IllegalStateException("eglGetDisplay failed")
        }
        val version = IntArray(2)
        if (!EGL14.eglInitialize(eglDisplay, version, 0, version, 1)) {
            throw IllegalStateException("eglInitialize failed")
        }
        Log.d(TAG, "EGL version: ${version.joinToString(".")}")
        val configAttributes = intArrayOf(
            EGL14.EGL_RENDERABLE_TYPE, EGL14.EGL_OPENGL_ES2_BIT,
            EGL14.EGL_RED_SIZE, 8,
            EGL14.EGL_GREEN_SIZE, 8,
            EGL14.EGL_BLUE_SIZE, 8,
            EGL14.EGL_ALPHA_SIZE, 8,
            EGL14.EGL_DEPTH_SIZE, 0,
            EGL14.EGL_CONFIG_CAVEAT, EGL14.EGL_NONE,
            EGL14.EGL_SURFACE_TYPE, EGL14.EGL_WINDOW_BIT,
            EGL14.EGL_NONE
        )
        val configs = arrayOfNulls<EGLConfig>(1)
        val numConfigs = IntArray(1)
        if (!EGL14.eglChooseConfig(eglDisplay, configAttributes, 0, configs, 0, 1, numConfigs, 0)) {
            throw IllegalStateException("eglChooseConfig failed")
        }
        if (numConfigs[0] <= 0 || configs[0] == null) {
            throw IllegalStateException("no EGLContext")
        }
        config = configs[0]!!

        val contextAttributes = intArrayOf(
            EGL14.EGL_CONTEXT_CLIENT_VERSION, 2,
            EGL14.EGL_NONE
        )
        val eglContext = EGL14.eglCreateContext(
            eglDisplay, configs[0], shareContext, contextAttributes, 0
        )
        if (eglContext == EGL14.EGL_NO_CONTEXT) {
            throw IllegalStateException("eglCreateContext failed")
        }

        display = eglDisplay
        context = eglContext
    }

    inline fun <T> withMakeCurrent(
        surface: EGLSurface = EGL14.EGL_NO_SURFACE,
        action: GlContext.() -> T
    ): T {
        if (!EGL14.eglMakeCurrent(display, surface, surface, context)) {
            throw IllegalStateException("eglMakeCurrent failed")
        }
        try {
            return this.action()
        } finally {
            // Don't bind context to thread for a long time
            EGL14.eglMakeCurrent(
                display,
                EGL14.EGL_NO_SURFACE,
                EGL14.EGL_NO_SURFACE,
                EGL14.EGL_NO_CONTEXT
            )
        }
    }

    fun createSurfaceTexture(
        externalTextureId: Int,
        width: Int,
        height: Int
    ): SurfaceTextureHolder = withMakeCurrent {
        when (getTarget(externalTextureId)) {
            GLES32.GL_TEXTURE_2D -> {
                val internalTextureId = createTextureInternal()
                val surfaceTexture = SurfaceTexture(internalTextureId).apply {
                    setDefaultBufferSize(width, height)
                }
                Texture2DSurfaceTexture(
                    externalTextureId,
                    width,
                    height,
                    surfaceTexture,
                    internalTextureId
                )
            }
            GLES11Ext.GL_TEXTURE_EXTERNAL_OES -> {
                val surfaceTexture = SurfaceTexture(externalTextureId).apply {
                    setDefaultBufferSize(width, height)
                }
                ExternalOESSurfaceTexture(
                    externalTextureId,
                    width,
                    height,
                    surfaceTexture
                )
            }
            else -> throw IllegalStateException("Invalid target of texture")
        }
    }

    fun releaseSurfaceTexture(holder: SurfaceTextureHolder) = withMakeCurrent {
        holder.surfaceTexture.release()
        holder.internalTextureId?.let { textureId ->
            deleteTextureInternal(textureId)
        }
    }

    fun createSurface(width: Int, height: Int): SurfaceHolder = withMakeCurrent {
        val textureId = createTextureInternal()
        val surfaceTexture = SurfaceTexture(textureId).apply {
            setDefaultBufferSize(width, height)
        }
        val surface = Surface(surfaceTexture)
        SurfaceHolder(textureId, surfaceTexture, surface)
    }

    fun releaseSurface(holder: SurfaceHolder) = withMakeCurrent {
        holder.surface.release()
        holder.surfaceTexture.release()
        deleteTextureInternal(holder.textureId)
    }

    private fun createTextureInternal(): Int = withMakeCurrent {
        val textureIdHolder = IntArray(1)
        GLES32.glGenTextures(textureIdHolder.size, textureIdHolder, 0)
        "glGenTextures".throwIfError()
        GLES32.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, textureIdHolder[0])
        "glBindTexture".throwIfError()
        textureIdHolder[0]
    }

    private fun deleteTextureInternal(textureId: Int) = withMakeCurrent {
        val textureIdHolder = intArrayOf(textureId)
        GLES32.glDeleteTextures(textureIdHolder.size, textureIdHolder, 0)
        "glDeleteTextures".throwIfError()
    }

    private fun getTarget(textureId: Int): Int {
        GLES32.glBindTexture(GLES11Ext.GL_TEXTURE_EXTERNAL_OES, textureId)
        if (GLES32.glGetError() == GLES32.GL_NO_ERROR) {
            Log.d(TAG, "target is TEXTURE_EXTERNAL_OES")
            return GLES11Ext.GL_TEXTURE_EXTERNAL_OES
        }
        GLES32.glBindTexture(GLES32.GL_TEXTURE_2D, textureId)
        if (GLES32.glGetError() == GLES32.GL_NO_ERROR) {
            Log.d(TAG, "target is TEXTURE_2D")
            return GLES32.GL_TEXTURE_2D
        }
        throw IllegalStateException("Can't bind texture")
    }

    override fun close() {
        if (isClosed) return
        isClosed = true
        EGL14.eglDestroyContext(display, context)
        EGL14.eglReleaseThread()
        EGL14.eglTerminate(display)
    }
}