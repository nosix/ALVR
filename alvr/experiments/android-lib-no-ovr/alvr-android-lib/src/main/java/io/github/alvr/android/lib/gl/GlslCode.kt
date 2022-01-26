package io.github.alvr.android.lib.gl

import org.intellij.lang.annotations.Language

val ATTRIB_VERTEX = FloatVertexAttribArray("a_Vertex", COORDS_2D)
const val UNIFORM_TEXTURE = "u_Texture"

@Language("glsl")
val VERTEX_SHADER = """
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

@Language("glsl")
val FFR_FRAGMENT_SHADER_COMMON_FORMAT = """
#extension GL_OES_EGL_image_external : enable
precision highp float;

uniform samplerExternalOES u_Texture;
varying vec2 v_TextureCoord;

const ivec2 TARGET_RESOLUTION = ivec2(%d, %d);
const ivec2 OPTIMIZED_RESOLUTION = ivec2(%d, %d);
const vec2 EYE_SIZE_RATIO = vec2(%f, %f);
const vec2 CENTER_SIZE = vec2(%f, %f);
const vec2 CENTER_SHIFT = vec2(%f, %f);
const vec2 EDGE_RATIO = vec2(%f, %f);

vec2 TextureToEyeUV(vec2 textureUV, bool isRightEye) {
    // flip distortion horizontally for right eye
    // left: x * 2; right: (1 - x) * 2
    return vec2((textureUV.x + float(isRightEye) * (1. - 2. * textureUV.x)) * 2., textureUV.y);
}

vec2 EyeToTextureUV(vec2 eyeUV, bool isRightEye) {
    // left: x / 2; right 1 - (x / 2)
    return vec2(eyeUV.x / 2. + float(isRightEye) * (1. - eyeUV.x), eyeUV.y);
}

"""

@Language("glsl")
val FFR_FRAGMENT_SHADER_DECOMPRESS_AXIS_ALIGNED = """
void main() {
    bool isRightEye = v_TextureCoord.x > 0.5;
    vec2 eyeUV = TextureToEyeUV(v_TextureCoord, isRightEye);

    vec2 alignedUV = eyeUV;

    vec2 c0 = (1.-CENTER_SIZE) / 2.;
    vec2 c1 = (EDGE_RATIO-1.) * c0 * (CENTER_SHIFT + 1.) / EDGE_RATIO;
    vec2 c2 = (EDGE_RATIO-1.) * CENTER_SIZE + 1.;

    vec2 loBound = c0 * (CENTER_SHIFT + 1.);
    vec2 hiBound = c0 * (CENTER_SHIFT - 1.) + 1.;
    vec2 underBound = vec2(
        alignedUV.x < loBound.x,
        alignedUV.y < loBound.y
    );
    vec2 inBound = vec2(
        loBound.x < alignedUV.x && alignedUV.x < hiBound.x,
        loBound.y < alignedUV.y && alignedUV.y < hiBound.y
    );
    vec2 overBound = vec2(
        alignedUV.x > hiBound.x,
        alignedUV.y > hiBound.y
    );

    vec2 d1 = (alignedUV - c1) * EDGE_RATIO / c2;

    vec2 center = d1;
    vec2 loBoundC = c0 * (CENTER_SHIFT + 1.) / c2;
    vec2 hiBoundC = c0 * (CENTER_SHIFT - 1.) / c2 + 1.;
    vec2 leftEdge = (-(c1+c2*loBoundC)/loBoundC+sqrt(((c1+c2*loBoundC)/loBoundC)*((c1+c2*loBoundC)/loBoundC)+4.*c2*(1.-EDGE_RATIO)/(EDGE_RATIO*loBoundC)*alignedUV))/(2.*c2*(1.-EDGE_RATIO))*(EDGE_RATIO*loBoundC);
    vec2 rightEdge = (-(c2-EDGE_RATIO*c1-2.*EDGE_RATIO*c2+c2*EDGE_RATIO*(1.-hiBoundC)+EDGE_RATIO)/(EDGE_RATIO*(1.-hiBoundC))+sqrt(((c2-EDGE_RATIO*c1-2.*EDGE_RATIO*c2+c2*EDGE_RATIO*(1.-hiBoundC)+EDGE_RATIO)/(EDGE_RATIO*(1.-hiBoundC)))*((c2-EDGE_RATIO*c1-2.*EDGE_RATIO*c2+c2*EDGE_RATIO*(1.-hiBoundC)+EDGE_RATIO)/(EDGE_RATIO*(1.-hiBoundC)))-4.*((c2*EDGE_RATIO-c2)*(c1-hiBoundC+hiBoundC*c2)/(EDGE_RATIO*(1.-hiBoundC)*(1.-hiBoundC))-alignedUV*(c2*EDGE_RATIO-c2)/(EDGE_RATIO*(1.-hiBoundC)))))/(2.*c2*(EDGE_RATIO-1.))*(EDGE_RATIO*(1.-hiBoundC));

    vec2 uncompressedUV = 
        underBound * leftEdge + 
        inBound * center + 
        overBound * rightEdge;

    gl_FragColor = texture2D(u_Texture, EyeToTextureUV(uncompressedUV * EYE_SIZE_RATIO, isRightEye));
}
"""
