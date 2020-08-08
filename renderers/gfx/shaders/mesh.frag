#version 450
#extension GL_GOOGLE_include_directive : require
#extension GL_ARB_separate_shader_objects : enable

#include "material.glsl"

layout(location = 0) in vec4 V;
layout(location = 1) in vec4 SSV;
layout(location = 2) in vec3 N;
layout(location = 3) flat in uint MID;
layout(location = 4) in vec2 TUV;
layout(location = 5) in vec3 T;
layout(location = 6) in vec3 B;

layout(set = 2, binding = 0) uniform Materials { Material Mat; };
layout(set = 2, binding = 1) uniform sampler2D AlbedoT;
layout(set = 2, binding = 2) uniform sampler2D NormalT;
layout(set = 2, binding = 3) uniform sampler2D RoughnessT;
layout(set = 2, binding = 4) uniform sampler2D EmissiveT;
layout(set = 2, binding = 5) uniform sampler2D SheenT;

layout(location = 0) out vec4 target0;

void main() {
    vec4 color = Mat.color;

    const uint flags = Mat.flags;
    if (HAS_DIFFUSE_MAP(flags)) {
        vec4 t_color = texture(AlbedoT, TUV).rgba;
        if (t_color.a < 0.5) {
            discard;
        }
        color = t_color;
    }

    target0 = color;
}