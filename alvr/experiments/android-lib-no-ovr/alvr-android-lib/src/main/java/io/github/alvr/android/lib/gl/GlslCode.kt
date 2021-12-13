package io.github.alvr.android.lib.gl

import org.intellij.lang.annotations.Language

val ATTRIB_VERTEX = FloatVertexAttribArray("a_Vertex", COORDS_2D)
const val UNIFORM_TEXTURE = "u_Texture"

@Language("glsl")
val PASS_THROUGH_VERTEX_SHADER = """
const vec2 madd = vec2(0.5,0.5);
attribute vec2 a_Vertex;
varying vec2 v_TextureCoord;
void main() {
    v_TextureCoord = a_Vertex.xy * madd + madd;
    gl_Position = vec4(a_Vertex.x, -a_Vertex.y, 0.0, 1.0);
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