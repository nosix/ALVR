package io.github.alvr.android.lib.gl

import org.intellij.lang.annotations.Language

@Language("glsl")
val PASS_THROUGH_VERTEX_SHADER = """
attribute vec4 a_Position;
attribute vec2 a_TextureCoord;
varying vec2 v_TextureCoord;
void main() {
    v_TextureCoord = a_TextureCoord;
    gl_Position = a_Position;
}
"""

@Language("glsl")
val PASS_THROUGH_FRAGMENT_SHADER = """
#extension GL_OES_EGL_image_external : enable
precision mediump float;
uniform samplerExternalOES u_Texture;
varying vec2 v_TextureCoord;
void main() {
    gl_FragColor = texture2D(u_Texture, v_TextureCoord);
}
"""