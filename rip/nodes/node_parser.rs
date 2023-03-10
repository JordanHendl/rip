extern crate runa;
extern crate json;
use std::{collections::{HashMap, HashSet, hash_map::Entry}, cell::RefCell, rc::Rc};
use runa::*;
use json::*;
use crate::common::data_bus::DataBus;

use super::{NodeCreateInfo, RipNode, starters, finishers, processors};

type Callback = fn(&NodeCreateInfo) -> Box<dyn RipNode + Send>;
fn get_reserved_strings() -> HashMap<String, String> {
  let src_dir = concat!(env!("CARGO_MANIFEST_DIR"));
  let mut reserved: HashMap<String, String> = Default::default();

  reserved.insert("${SRC_DIR}".to_string(), src_dir.to_string());
  return reserved;
}

fn get_starter_functors() -> HashMap<String, Callback> {
  let mut functors: HashMap<String, Callback> = Default::default();
  functors.insert("pattern".to_string(), starters::Pattern::new);
  functors.insert("image_load".to_string(), starters::ImageLoad::new);
  return functors;
}

fn get_finisher_functors() -> HashMap<String, Callback> {
  let mut functors: HashMap<String, Callback> = Default::default();
  functors.insert("image_write".to_string(), finishers::ImageWrite::new);
  functors.insert("display".to_string(), finishers::Display::new);
  return functors;
} 

fn get_functors() -> HashMap<String, Callback> {
  let mut functors: HashMap<String, Callback> = Default::default();
  functors.insert("arithmetic".to_string(), processors::Arithmetic::new);
  functors.insert("tonemap".to_string(), processors::Tonemap::new);
  functors.insert("monochrome".to_string(), processors::Monochrome::new);
  functors.insert("inverse".to_string(), processors::Inverse::new);
  functors.insert("threshold".to_string(), processors::Threshold::new);
  functors.insert("adaptive_threshold".to_string(), processors::AdaptiveThreshold::new);
  functors.insert("blur".to_string(), processors::Blur::new);
  functors.insert("chroma_key".to_string(), processors::ChromaKey::new);
  functors.insert("crop".to_string(), processors::Crop::new);
  functors.insert("overlay".to_string(), processors::Overlay::new);
  functors.insert("object_highlight".to_string(), processors::ObjectHighlight::new);
  functors.insert("color_space_conversion".to_string(), processors::ColorSpaceConversion::new);
  functors.insert("transform".to_string(), processors::Transform::new);
  functors.insert("connected_components".to_string(), processors::ConnectedComponents::new);
  return functors;
}

fn find_connections(starter_ids: &HashMap<String, usize>, node_ids: &HashMap<String, usize>, finisher_ids: &HashMap<String, usize>,
  _starters: &JsonValue, nodes: &JsonValue, finishers: &JsonValue) -> (HashMap<u32, Vec<u32>>, Vec<u32>) {
  // Now to find execution order
  let mut execution_order: Vec<u32> = Vec::new();
  let mut inserted_nodes: HashSet<String> = HashSet::new();
  let mut connections: HashMap<u32, Vec<u32>> = HashMap::new();
  
  // We can safetly execute all starters as they have no preconditions
  for starter in starter_ids {
    execution_order.push(*starter.1 as u32);
    inserted_nodes.insert(starter.0.clone());
  }
  let mut num_inserted_nodes = 0;
  
  let mut insert_fn = |node: (&str, &JsonValue) , ids: &HashMap<String, usize>| {
    assert!(node.1.has_key("input"));
    
    let mut num_deps = 1;
    let entry = ids.get(node.0).unwrap();
    if !inserted_nodes.contains(&node.0.to_string()) {
      let mut has_all_deps = 0;
      let mut dep_names:Vec<String> = Vec::new();
      
      let raw_input = &node.1["input"];
      
      if raw_input.is_array() {
        num_deps = raw_input.len();
        for i in 0..raw_input.len() {
          let mut input_name = raw_input[i].as_str().unwrap().to_string();
          let is_an_output = input_name.find(".output");
          input_name = input_name.chars().take(is_an_output.unwrap()).collect();

          if inserted_nodes.contains(&input_name) {
            has_all_deps += 1;
            dep_names.push(input_name.clone());
          }
        }
      } else {
        let input = raw_input.as_str().as_ref().unwrap().to_string().clone();
        let is_an_output = input.find(".output");
        if is_an_output.is_none() {
          panic!("Some node doesn't have it's input specified correctly. Inputs should come in the form 'input: \"other_node.output\"'");
        }
      
        // Find the node name from the input
        let input_name: String = input.chars().take(is_an_output.unwrap()).collect();
        
        // This node depends on a starter, so we can safetly just have them execute whenever.
        if inserted_nodes.contains(&input_name) {
          has_all_deps += 1;
          dep_names.push(input_name.clone());
        }
      }
      
      if has_all_deps == num_deps {
        execution_order.push(*entry as u32);
        inserted_nodes.insert(node.0.to_string());
        
        for parent_name in dep_names {
          // Bro wtf is this
          let parent_id = if starter_ids.contains_key(&parent_name.to_string()) 
          {*starter_ids.get(&parent_name.to_string()).unwrap() as u32} else 
          {*node_ids.get(&parent_name.to_string()).unwrap() as u32};
          
          let self_id = (execution_order.len() - 1) as u32;
          // Map connections so we know where to send data as we execute nodes.          
          match connections.entry(self_id) {
            Entry::Occupied(mut o) => {
            println!("Adding connection between node {} -> {}", parent_id, self_id);
            o.get_mut().push(parent_id);
          },
          Entry::Vacant(v) => {
            println!("Adding connection between node {} -> {}", parent_id, execution_order.len() - 1);
            let mut vec: Vec<u32> = Vec::new();
            vec.push(parent_id);
            v.insert(vec);
          },
          }
        }
        println!("Found all deps for {}!", node.0);
        return true;
      }
    }
    return false;
  };
  
  // Loop until we get all the nodes inserted.
  while num_inserted_nodes != node_ids.len() {
    for node in nodes.entries() {
      if insert_fn(node, &node_ids) {
        num_inserted_nodes += 1;
      }
    }
  }
  
  num_inserted_nodes = 0;
  while num_inserted_nodes != finisher_ids.len() {
    for node in finishers.entries() {
      if insert_fn(node, &finisher_ids) {
        num_inserted_nodes += 1;
      }
    }
  }
  
  return (connections, execution_order);
}

fn configure_nodes(json: &JsonValue) {
  let data_bus: DataBus = Default::default();
  let reserved_strings = get_reserved_strings();
  for node in json.entries() {
    for config in node.1.entries() {
      if config.0.ne("type") {
        let key = node.0.to_string() + &"::".to_string() + config.0;
        println!("Parsing configuration option {}", key);
        if config.1.is_boolean() {
          data_bus.send(&key, &config.1.as_bool().unwrap());
        } else if config.1.is_string() {

          let mut value = config.1.as_str().unwrap().to_string().clone();
          for string in &reserved_strings {
            value = value.replace(string.0, string.1);
          }
          
          
          data_bus.send(&key, &value);
        } else if config.1.is_number() {
          if config.1.as_u32().as_ref().is_some() {
            data_bus.send(&key, config.1.as_u32().as_ref().unwrap());
          }

          if config.1.as_f32().as_ref().is_some() {
            data_bus.send(&key, config.1.as_f32().as_ref().unwrap());
          }
        }
      }
    }
  }
}

pub fn parse_json(interface: &Rc<RefCell<gpu::GPUInterface>>, json_data: &str) -> 
(Vec<Box<dyn RipNode + Send>>, Vec<u32>, HashMap<u32, Vec<u32>>, (u32, u32), (String, String)) {

  let starter_functors = get_starter_functors();
  let node_functors = get_functors();
  let finisher_functors = get_finisher_functors();

  let mut created_nodes:  Vec<Box<dyn RipNode + Send>> = Vec::new();
  let mut network_info: (String, String) = ("127.0.0.1".to_string(), "5555".to_string());
  let mut starter_ids: HashMap<String, usize> = HashMap::new();
  let mut node_ids: HashMap<String, usize> = HashMap::new();
  let mut finisher_ids: HashMap<String, usize> = HashMap::new();

  println!("Loaded json file: \n{}", json_data);
  let root = json::parse(json_data).expect("Unable to parse JSON configuration!");
  assert!(root.has_key("starters"), "Failed to find any pipeline starters in the configuration!");
  assert!(root.has_key("finishers"), "Failed to find any pipeline finishers in the configuration!");

  let starters = &root["starters"];
  let nodes = &root["imgproc"];
  let finishers = &root["finishers"];
  
  let mut xdim = 1280;
  let mut ydim = 1024;
  if root.has_key("dimensions") {
    let dims = &root["dimensions"];
    assert!(dims.is_array());
    xdim = dims[0].as_u32().unwrap();
    ydim = dims[1].as_u32().unwrap();
  }

  if root.has_key("network") {
    let network = &root["network"];
    if network.has_key("ip") {
      network_info.0 = network["ip"].as_str().unwrap().to_string().clone();
    }

    if network.has_key("port") {
      network_info.1 = network["port"].as_str().unwrap().to_string().clone();
    }
  }
  let node_handler = |node: (&str, &JsonValue), functors: &HashMap<String, Callback>| {
    let name = node.0;
    let type_name = if node.1.has_key("type") {node.1["type"].as_str()} else {Some(node.0)}.unwrap();
    println!("JSON: Parsing {} of type {}...", name, type_name);

    if functors.contains_key(type_name) {
      let create_info = NodeCreateInfo {
        interface: interface.clone(),
        name: name.to_string(),
      };
      let node = functors[type_name](&create_info);
      return Some(node);
    }
    return None;
  };

  for node in starters.entries() {
    let created_node = node_handler(node, &starter_functors);
    if created_node.is_some() {
      let n = created_node.unwrap();
      let name = n.name().clone();
      created_nodes.push(n);
      starter_ids.insert(name, created_nodes.len() - 1);
    }
  }
  
  for node in nodes.entries() {
    let created_node = node_handler(node, &node_functors);
    if created_node.is_some() {
      let n = created_node.unwrap();
      let name = n.name().clone();
      created_nodes.push(n);
      node_ids.insert(name, created_nodes.len() - 1);
    }
  }

  for node in finishers.entries() {
    let created_node = node_handler(node, &finisher_functors);
    if created_node.is_some() {
      let n = created_node.unwrap();
      let name = n.name().clone();
      created_nodes.push(n);
      finisher_ids.insert(name, created_nodes.len() - 1);
    }
  }

      
  configure_nodes(starters);
  configure_nodes(nodes);
  configure_nodes(finishers);

  let (connections, execution_order) = find_connections(
    &starter_ids, 
    &node_ids, 
    &finisher_ids, 
    &starters, 
    &nodes, 
    &finishers);


  return (created_nodes, execution_order, connections, (xdim, ydim), network_info);
}