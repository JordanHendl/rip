extern crate runa;
use std::cell::RefCell;
use std::rc::Rc;
use crate::common;

use super::NodeCreateInfo;
use super::SwsppNode;
use super::common::*;
use runa::*;

///////////////////////////////////////////////////
/// Structure declarations
///////////////////////////////////////////////////

#[derive(Default)]
struct IntensifyData {
  image: Option<gpu::ImageView>,
  pipeline: gpu::ComputePipeline,
  bind_group: gpu::BindGroup,
}

pub struct Intensify {
  interface: Rc<RefCell<gpu::GPUInterface>>,
  data: IntensifyData,
  data_bus: crate::common::DataBus,
  name: String,
}

///////////////////////////////////////////////////
/// Implementations
///////////////////////////////////////////////////

// Need send to send through threads safely
unsafe impl Send for Intensify {}

// Implementations specific to this node
impl Intensify {
  pub fn new(info: &NodeCreateInfo) -> Box<dyn SwsppNode + Send> {
    println!("Creating node {} as an Intensify node!", info.name);
    let mut obj = Box::new(Intensify {
      interface: info.interface.clone(),
      data: Default::default(),
      data_bus: Default::default(),
      name: info.name.to_string(),
    });

    let raw_shader = common::to_u32_slice(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/target/shaders/intensify.spirv")).as_ref());
    let info = gpu::ComputePipelineCreateInfo::builder()
    .gpu(0)
    .shader(raw_shader)
    .name(&info.name)
    .build();

    let pipeline = gpu::ComputePipeline::new(&obj.interface, &info);
    obj.data.bind_group = pipeline.bind_group();
    obj.data.pipeline = pipeline;
    return obj;
  }
}

// Base class implementations
impl SwsppNode for Intensify {
  fn execute(& mut self, cmd: & mut gpu::CommandList) {
    println!("Executing Node {}", self.name);
    let (x, y, z) = self.data.image.as_ref().unwrap().get_compute_groups(32, 32, 1);
    cmd.bind_compute(&self.data.pipeline);
    cmd.bind(&self.data.bind_group);
    cmd.dispatch(x, y, z);

    self.data.image = None;
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
    return "Intensify".to_string();
  }
}