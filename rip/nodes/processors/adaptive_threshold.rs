extern crate runa;
use std::cell::RefCell;
use std::rc::Rc;
use crate::common;

use super::NodeCreateInfo;
use super::RipNode;
use super::common::*;
use runa::*;

///////////////////////////////////////////////////
/// Structure declarations
///////////////////////////////////////////////////

#[repr(C)]
struct AdaptiveThresholdConfig {
  radius: u32,
  mode: u32,
}

#[derive(Default)]
struct AdaptiveThresholdData {
  image: Option<gpu::ImageView>,
  config: Option<gpu::Vector<AdaptiveThresholdConfig>>,
  pipeline: gpu::ComputePipeline,
  bind_group: gpu::BindGroup,
}

pub struct AdaptiveThreshold {
  interface: Rc<RefCell<gpu::GPUInterface>>,
  data: AdaptiveThresholdData,
  data_bus: crate::common::DataBus,
  name: String,
}

///////////////////////////////////////////////////
/// Implementations
///////////////////////////////////////////////////

// Need send to send through threads safely
unsafe impl Send for AdaptiveThreshold {}

impl Default for AdaptiveThresholdConfig {
  fn default() -> Self {
      return AdaptiveThresholdConfig { mode: 0, radius: 4}
  }
}

// Implementations specific to this node
impl AdaptiveThreshold {
  pub fn set_radius(& mut self, input: &u32) {
    println!("Setting radius {} for node {}", input, self.name);

    let mapped = unsafe{self.data.config.as_mut().unwrap().map()};
    mapped[0].radius = *input;
    unsafe{self.data.config.as_mut().unwrap().unmap()};
  }

  pub fn set_mode(& mut self, input: &String) {
    println!("Setting mode {} for node {}", input, self.name);
    let mut mode = 0;
    match input.as_str() {
      "stddev" => mode = 0,
      _ => {},
    }

    let mapped = unsafe{self.data.config.as_mut().unwrap().map()};
    mapped[0].mode = mode;
    unsafe{self.data.config.as_mut().unwrap().unmap()};
  }

  pub fn new(info: &NodeCreateInfo) -> Box<dyn RipNode + Send> {
    let mut obj = Box::new(AdaptiveThreshold {
      interface: info.interface.clone(),
      data: Default::default(),
      data_bus: Default::default(),
      name: info.name.to_string(),
    });

    let raw_shader = common::to_u32_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/target/shaders/adaptive_threshold.spirv")).as_ref());
    let info = gpu::ComputePipelineCreateInfo::builder()
    .gpu(0)
    .shader(raw_shader)
    .name(&info.name)
    .build();

    let buff_info = gpu::BufferCreateInfo::builder()
    .gpu(0)
    .size(1)
    .build();

    let pipeline = gpu::ComputePipeline::new(&obj.interface, &info);
    obj.data.bind_group = pipeline.bind_group();
    obj.data.pipeline = pipeline;

    let default_config: AdaptiveThresholdConfig = Default::default();
    obj.data.config = Some(gpu::Vector::new(&obj.interface, &buff_info));
    obj.data.config.as_mut().unwrap().upload(std::slice::from_ref(&default_config));

    let mut bus: DataBus = Default::default();
    let name = info.name.clone();
    bus.add_object_subscriber(&(name.clone() + "::mode"), obj.as_mut(), AdaptiveThreshold::set_mode);
    bus.add_object_subscriber(&(name.clone() + "::radius"), obj.as_mut(), AdaptiveThreshold::set_radius);
    obj.data_bus = bus;


    obj.data.bind_group.bind_vector("config", obj.data.config.as_ref().unwrap());
    return obj;
  }
}

// Base class implementations
impl RipNode for AdaptiveThreshold {
  fn execute(& mut self, cmd: & mut gpu::CommandList) {
    println!("Executing Node {}", self.name);
    let (x, y, z) = self.data.image.as_ref().unwrap().get_compute_groups(32, 32, 1);
    cmd.bind_compute(&self.data.pipeline);
    cmd.bind(&self.data.bind_group);
    cmd.dispatch(x, y, z);
    cmd.image_write_barrier(self.data.image.as_ref().unwrap());
  }

  fn input(& mut self, image: &gpu::ImageView) {
    self.data.bind_group.bind_image_view("input_tex", image);
  }

  fn assign(& mut self, view: &gpu::ImageView) {
    self.data.image = Some(view.clone());
    self.data.bind_group.bind_image_view("output_tex", &self.data.image.as_ref().unwrap());
  }


  fn name(&self) -> String {
    return self.name.clone();
  }

  fn node_type(&self) -> String {
    return "monochrome".to_string();
  }
}