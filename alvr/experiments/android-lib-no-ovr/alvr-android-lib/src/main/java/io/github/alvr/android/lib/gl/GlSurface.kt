package io.github.alvr.android.lib.gl

import android.opengl.EGL14
import android.opengl.EGLSurface
import android.view.Surface

class GlSurface(
    val context: GlContext,
    surface: Surface
) {
    val surface: EGLSurface

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

        this.surface = eglSurface
    }

    fun release() {
        EGL14.eglDestroySurface(context.display, surface)
    }

    inline fun <T> withGlContext(action: GlContext.() -> T): T {
        return context.withMakeCurrent(surface, action)
    }
}