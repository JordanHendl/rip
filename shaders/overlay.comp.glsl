#version 450 core
#extension GL_ARB_separate_shader_objects : enable
#extension GL_GOOGLE_include_directive    : enable

#define BLOCK_SIZE_X 32
#define BLOCK_SIZE_Y 32
#define BLOCK_SIZE_Z 1

layout(local_size_x = BLOCK_SIZE_X, local_size_y = BLOCK_SIZE_Y, local_size_z = BLOCK_SIZE_Z) in;

#include "include/color_space.glsl"

layout(binding = 0, rgba32f) coherent restrict readonly  uniform image2D input_tex_0;
layout(binding = 1, rgba32f) coherent restrict readonly  uniform image2D input_tex_1;
layout(binding = 2, rgba32f) coherent restrict writeonly uniform image2D output_tex;

layout(binding = 3) uniform OverlayConfig {
  uint dummy;
} config;

void main() {
  const ivec2 tex_coords = ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y);
  vec4 color = imageLoad(input_tex_0, tex_coords);
  vec4 color2 = imageLoad(input_tex_1, tex_coords);

  vec3 ca = color.rgb;
  vec3 cb = color2.rgb;

  vec3 out_color = (ca * vec3(color.a)) + (cb * vec3(1.0 - color.a));
  imageStore(output_tex, tex_coords, vec4(out_color, 1.0)); 
}
