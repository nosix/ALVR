package io.github.alvr.android.lib

import android.graphics.SurfaceTexture
import android.opengl.EGL14
import android.opengl.EGLConfig
import android.opengl.EGLContext
import android.opengl.EGLDisplay
import android.opengl.EGLSurface
import android.opengl.GLES20
import android.util.Log
import android.util.Size
import android.view.Surface
import javax.microedition.khronos.opengles.GL10

class EGLSurfaceHelper(
    shareContext: EGLContext = EGL14.EGL_NO_CONTEXT,
    surface: Surface? = null
) : AutoCloseable {

    companion object {
        private val TAG = EGLSurfaceHelper::class.simpleName

        private val P_BUFFER_SIZE = Size(1, 1)

        private fun Int.toErrorString(): String = when (this) {
            GL10.GL_NO_ERROR -> "no error"
            GL10.GL_INVALID_ENUM -> "invalid enum"
            GL10.GL_INVALID_VALUE -> "invalid value"
            GL10.GL_INVALID_OPERATION -> "invalid operation"
            GL10.GL_STACK_OVERFLOW -> "stack overflow"
            GL10.GL_STACK_UNDERFLOW -> "stack underflow"
            GL10.GL_OUT_OF_MEMORY -> "out of memory"
            else -> "unknown"
        }
    }

    private val mDisplay: EGLDisplay
    private val mContext: EGLContext
    private val mSurface: EGLSurface

    private var isClosed: Boolean = false

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
        val eglSurface = if (surface != null) {
            val attributes = intArrayOf(
                EGL14.EGL_NONE
            )
            EGL14.eglCreateWindowSurface(
                eglDisplay, configs[0], surface, attributes, 0
            )
        } else {
            val attributes = intArrayOf(
                EGL14.EGL_WIDTH, P_BUFFER_SIZE.width,
                EGL14.EGL_HEIGHT, P_BUFFER_SIZE.height,
                EGL14.EGL_NONE
            )
            EGL14.eglCreatePbufferSurface(
                eglDisplay, configs[0], attributes, 0
            )
        }.also {
            if (it == EGL14.EGL_NO_SURFACE) {
                throw IllegalStateException("eglCreatePBufferSurface failed")
            }
        }
        mDisplay = eglDisplay
        mContext = eglContext
        mSurface = eglSurface
        Log.i(TAG, Thread.currentThread().name)
    }

    private inline fun <T> withMakeCurrent(action: EGLSurfaceHelper.() -> T): T {
        if (!EGL14.eglMakeCurrent(mDisplay, mSurface, mSurface, mContext)) {
            throw IllegalStateException("eglMakeCurrent failed")
        }
        try {
            return this.action()
        } finally {
            // Don't bind context to thread for a long time
            EGL14.eglMakeCurrent(
                mDisplay,
                EGL14.EGL_NO_SURFACE,
                EGL14.EGL_NO_SURFACE,
                EGL14.EGL_NO_CONTEXT
            )
        }
    }

    fun createSurface(
        textureId: Int,
        width: Int,
        height: Int,
        onFrameAvailable: () -> Unit
    ): SurfaceHolder {
        val surfaceTexture = SurfaceTexture(textureId)
        surfaceTexture.setDefaultBufferSize(width, height)
        surfaceTexture.setOnFrameAvailableListener {
            onFrameAvailable()
        }
        val surface = Surface(surfaceTexture)
        return SurfaceHolder(this, textureId, surfaceTexture, surface)
    }

    fun createSurface(width: Int, height: Int): SurfaceHolder {
        val textureId = createTexture()
        val surfaceTexture = SurfaceTexture(textureId)
        surfaceTexture.setDefaultBufferSize(width, height)
        val surface = Surface(surfaceTexture)
        return SurfaceHolder(this, textureId, surfaceTexture, surface)
    }

    private fun createTexture(): Int {
        withMakeCurrent {
            val textureIdHolder = IntArray(1)
            GLES20.glGenTextures(textureIdHolder.size, textureIdHolder, 0)
            val error = GLES20.glGetError()
            if (error != GLES20.GL_NO_ERROR) {
                throw IllegalStateException("glGenTextures: ${error.toErrorString()}")
            }
            return textureIdHolder[0]
        }
    }

    private fun deleteTexture(textureId: Int) {
        withMakeCurrent {
            val textureIdHolder = IntArray(1)
            textureIdHolder[0] = textureId
            GLES20.glDeleteTextures(textureIdHolder.size, textureIdHolder, 0)
            val error = GLES20.glGetError()
            if (error != GLES20.GL_NO_ERROR) {
                throw IllegalStateException("glDeleteTextures: ${error.toErrorString()}")
            }
        }
    }

    private fun render(textureId: Int) {
        Log.i(TAG, Thread.currentThread().name)
        withMakeCurrent {
            GLES20.glViewport(0, 0, 512, 1024)
            GLES20.glClearColor(1f, 0f, 1f, 1f)
            GLES20.glClear(GLES20.GL_COLOR_BUFFER_BIT)
            GLES20.glFlush()
            EGL14.eglSwapBuffers(mDisplay, mSurface)
        }
    }

    @Synchronized
    override fun close() {
        if (isClosed) return
        isClosed = true
        EGL14.eglDestroySurface(mDisplay, mSurface)
        EGL14.eglDestroyContext(mDisplay, mContext)
        EGL14.eglReleaseThread()
        EGL14.eglTerminate(mDisplay)
    }

    class SurfaceHolder(
        private val eglSurface: EGLSurfaceHelper,
        val textureId: Int,
        val surfaceTexture: SurfaceTexture,
        val surface: Surface
    ) : AutoCloseable {

        override fun close() {
            surface.release()
            surfaceTexture.release()
            eglSurface.deleteTexture(textureId)
        }

        fun render() {
            eglSurface.render(textureId)
        }
    }
}