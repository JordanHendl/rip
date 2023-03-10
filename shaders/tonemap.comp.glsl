#version 450 core
#extension GL_ARB_separate_shader_objects : enable
#extension GL_GOOGLE_include_directive    : enable

#define BLOCK_SIZE_X 32 
#define BLOCK_SIZE_Y 32 
#define BLOCK_SIZE_Z 1 
#define MAX_BINS 1024

layout( local_size_x = BLOCK_SIZE_X, local_size_y = BLOCK_SIZE_Y, local_size_z = BLOCK_SIZE_Z ) in ; 
layout( binding = 0, rgba32f ) coherent restrict readonly uniform image2D input_tex;
layout( binding = 1, rgba32f ) coherent restrict writeonly uniform image2D output_tex;

layout(binding = 2) uniform HistogramConfig {
  uint num_bins;
  float min_rad;
  float max_rad;
} hist_config;

layout(binding = 3) readonly buffer HistogramData {
  uint histogram[];
} hist_data;

layout(binding = 4) readonly buffer TonemapData {
  float tonecurve[];
} data;

layout(binding = 5) readonly buffer TonemapConfig {
  uint mode;
} config;

vec3 sample_tonecurve(vec3 pix, uint bin) {
  return vec3(data.tonecurve[bin]) * pix;
}

void main() {
  const uint mode = config.mode;
  const ivec2 coords = ivec2(gl_GlobalInvocationID.xy);  
  const vec4 radiance = imageLoad(input_tex, coords);
  vec3 pix = radiance.rgb;
  uint bin = clamp(uint((radiance.r - hist_config.min_rad)/(hist_config.max_rad-hist_config.min_rad) * hist_config.num_bins), 0, hist_config.num_bins-1);
  pix = sample_tonecurve(pix, bin);
  imageStore(output_tex, coords, vec4(pix, 1.0));
}