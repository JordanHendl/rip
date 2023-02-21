#version 450 core
#extension GL_ARB_separate_shader_objects : enable
#extension GL_GOOGLE_include_directive    : enable

#define BLOCK_SIZE_X 32 
#define BLOCK_SIZE_Y 32 
#define BLOCK_SIZE_Z 1 
#define MAX_BINS 1024

layout( local_size_x = BLOCK_SIZE_X, local_size_y = BLOCK_SIZE_Y, local_size_z = BLOCK_SIZE_Z ) in ; 
layout( binding = 0, rgba32f ) coherent restrict readonly  uniform image2D input_tex;

layout(binding = 1) uniform HistogramConfig {
  int num_bins;
  int img_width;
  int img_height;
  float min_rad;
  float max_rad;
} config;

layout(binding = 2) writeonly buffer HistogramData {
  uint histogram[];
} data;

shared uint partial_hist[MAX_BINS];

void main()
{
  const ivec2 coords = ivec2(gl_GlobalInvocationID.xy);
  const ivec2 dim = imageSize(input_tex);
  uint global_id = gl_GlobalInvocationID.x * (gl_GlobalInvocationID.y * dim.x);
  uint local_id = gl_LocalInvocationIndex;
  
  if(local_id < config.num_bins) {
    partial_hist[local_id] = 0;
  }
  
  memoryBarrier();
  barrier();
  
  // load image and store into shared memory partial hist.
  const vec4 radiance = imageLoad(input_tex, coords);
  const float pix = radiance.r;
  int bin = clamp(int((pix - config.min_rad)/(config.max_rad-config.min_rad) * config.num_bins), 0, config.num_bins-1);
  if(radiance.a > 0 && pix > config.min_rad && pix < config.max_rad) {
    atomicAdd(data.histogram[bin], 1);  
  }
}