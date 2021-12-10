package io.github.alvr.android.lib.gl

import android.opengl.EGL14
import android.opengl.EGLSurface
import android.opengl.GLES32
import android.view.Surface

class GlSurface(
    val context: GlContext,
    surface: Surface
) {
    private val mSurface: EGLSurface

    init {
        val attributes = intArrayOf(
            EGL14.EGL_NONE
        )
        val eglSurface = EGL14.eglCreateWindowSurface(
            context.display, context.config, surface, attributes, 0
        )
        if (eglSurface == EGL14.EGL_NO_SURFACE) {
            throw IllegalStateException("eglCreateWindowSurface failed")
        }

        mSurface = eglSurface
    }

    fun release() {
        EGL14.eglDestroySurface(context.display, mSurface)
    }

    fun render(frame: SurfaceHolder) {
        context.withMakeCurrent(mSurface) {
            GLES32.glViewport(0, 0, 512, 1024)
            GLES32.glClearColor(1f, 0f, 1f, 1f)
            GLES32.glClear(GLES32.GL_COLOR_BUFFER_BIT)
            GLES32.glFlush()
            EGL14.eglSwapBuffers(display, mSurface)
        }
    }
}