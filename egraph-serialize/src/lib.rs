#[cfg(feature = "graphviz")]
mod graphviz;

mod algorithms;

use core::panic;
use std::sync::Arc;

use indexmap::{map::Entry, IndexMap};
use once_cell::sync::OnceCell;
use ordered_float::NotNan;

pub type Cost = NotNan<f64>;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
pub struct NodeId(pub [u32; 2]);

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NodeId_old(Arc<str>);

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Copy)]
pub struct ClassId(pub u32);


mod id_impls {
    use super::*;


    // impl AsRef<str> for NodeId {
    //     fn as_ref(&self) -> &str {
    //         &self.0
    //     }
    // }

    impl ClassId {
        pub fn return_value(&self) -> u32 {
            self.0
        }
    }
    
    impl std::fmt::Display for NodeId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Output in "a.b" format
            write!(f, "{}.{}", self.0[0], self.0[1])
        }
    }

    // impl std::fmt::Display for NodeId {
    //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //         write!(f, "{}", self.0)
    //     }
    // }
    // impl AsRef<str> for ClassId {
    //     fn as_ref(&self) -> &str {
    //         &self.0
    //     }
    // }

    // impl<S: Into<String>> From<S> for NodeId {
    //     fn from(s: S) -> Self {
    //         Self(s.into().into())
    //     }
    // }

    impl From<(u32, u32)> for NodeId {
        fn from(value: (u32, u32)) -> Self {
            NodeId([value.0, value.1])
        }
    }
    

    impl From<u32> for ClassId { 
        fn from(val: u32) -> Self {
            ClassId(val)
        }
    }
    

    impl std::fmt::Display for ClassId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct EGraph {
    pub nodes: IndexMap<NodeId, Node>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub root_eclasses: Vec<ClassId>,
    // Optional mapping of e-class ids to some additional data about the e-class
    #[cfg_attr(feature = "serde", serde(default))]
    pub class_data: IndexMap<ClassId, ClassData>,
    #[cfg_attr(feature = "serde", serde(skip))]
    once_cell_classes: OnceCell<IndexMap<ClassId, Class>>,
}

impl EGraph {
    /// Adds a new node to the egraph
    ///
    /// Panics if a node with the same id already exists
    pub fn add_node(&mut self, node_id: impl Into<NodeId>, node: Node) {
        match self.nodes.entry(node_id.into()) {
            Entry::Occupied(e) => {
                panic!(
                    "Duplicate node with id {key:?}\nold: {old:?}\nnew: {new:?}",
                    key = e.key(),
                    old = e.get(),
                    new = node
                )
            }
            Entry::Vacant(e) => e.insert(node),
        };
    }

    pub fn nid_to_cid(&self, node_id: &NodeId) -> &ClassId {
        &self[node_id].eclass
    }

    pub fn nid_to_class(&self, node_id: &NodeId) -> &Class {
        &self[&self[node_id].eclass]
    }

    /// Groups the nodes in the e-graph by their e-class
    ///
    /// This is *only done once* and then the result is cached.
    /// Modifications to the e-graph will not be reflected
    /// in later calls to this function.
    pub fn classes(&self) -> &IndexMap<ClassId, Class> {
        self.once_cell_classes.get_or_init(|| {
            let mut classes = IndexMap::new();
            for (node_id, node) in &self.nodes {
                classes
                    .entry(node.eclass.clone())
                    .or_insert_with(|| Class {
                        id: node.eclass.clone(),
                        nodes: vec![],
                    })
                    .nodes
                    .push(node_id.clone())
            }
            classes
        })
    }

    #[cfg(feature = "serde")]
    pub fn from_json_file(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let egraph: Self = serde_json::from_reader(std::io::BufReader::new(file))?;
        Ok(egraph)
    }

    pub fn from_Data(data: &Data) -> std::io::Result<Self> {
        let Data { nodes, root_eclasses } = data;
        let mut egraph = Self {
            nodes: nodes.clone(),
            root_eclasses: root_eclasses.clone(),
            ..Default::default()
        };
        egraph.once_cell_classes = Default::default();
        Ok(egraph)
    }

    #[cfg(feature = "serde")]
    pub fn to_json_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        let file = std::fs::File::create(path)?;
        serde_json::to_writer_pretty(std::io::BufWriter::new(file), self)?;
        Ok(())
    }

    #[cfg(feature = "serde")]
    pub fn test_round_trip(&self) {
        let json = serde_json::to_string_pretty(&self).unwrap();
        let egraph2: EGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(self, &egraph2);
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
pub struct Data_old {
    pub nodes: IndexMap<NodeId_old, Node_old>,
    pub root_eclasses: Vec<ClassId>,
}

impl Data_old {
    pub fn from_json_file(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let Data_old: Self = serde_json::from_reader(std::io::BufReader::new(file))?;
        Ok(Data_old)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[derive(Clone)]
pub struct Data {
    pub nodes: IndexMap<NodeId, Node>,
    pub root_eclasses: Vec<ClassId>,
}

impl Data {
    pub fn from_json_file(path: impl AsRef<std::path::Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(path)?;
        let data_old: Data_old = serde_json::from_reader(std::io::BufReader::new(&file))?;
        
        let mut new_nodes = IndexMap::new();
        for (old_id, old_node) in data_old.nodes.into_iter() {
            let new_id = convert_nodeid_old(&old_id);
            // For internal node ids, convert them as well
            let new_node = Node {
                op: old_node.op,
                id: new_id.clone(),
                children: old_node.children, // children and eclass remain unchanged
                eclass: old_node.eclass,
                cost: old_node.cost,
            };
            new_nodes.insert(new_id, new_node);
        }
        
        let data = Data {
            nodes: new_nodes,
            root_eclasses: data_old.root_eclasses,
        };
        Ok(data)
    }

    pub fn to_json_file(&self, path: impl AsRef<std::path::Path>) -> std::io::Result<()> {
        // Iterate through self.nodes, convert each node to old format
        let mut nodes_old = IndexMap::new();
        for (node_id, node) in &self.nodes {
            let old_id = convert_nodeid_to_old(node_id);
            let node_old = Node_old {
                op: node.op.clone(),
                id: old_id.clone(), // Also convert internal node id
                children: node.children.clone(), // Other fields remain unchanged
                eclass: node.eclass.clone(),
                cost: node.cost,
            };
            nodes_old.insert(old_id, node_old);
        }
        let data_old = Data_old {
            nodes: nodes_old,
            root_eclasses: self.root_eclasses.clone(),
        };
        // Serialize to JSON string and write to file
        let new_file_content =
            serde_json::to_string_pretty(&data_old).expect("Unable to serialize JSON");
        println!("{}", path.as_ref().display());
        std::fs::write(path, new_file_content).expect("Unable to write file");
        Ok(())
    }
}

fn convert_nodeid_to_old(node_id: &NodeId) -> NodeId_old {
    // Generate string using "a.b" format and wrap as Arc<str>
    NodeId_old(Arc::from(format!("{}.{}", node_id.0[0], node_id.0[1])))
}

fn convert_nodeid_old(old: &NodeId_old) -> NodeId {
    // Assume NodeId_old internally stores strings in "a.b" format
    let s: &str = &old.0;
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 2 {
        panic!("Invalid NodeId_old format: {}", s);
    }
    let a = parts[0].parse::<u32>().expect("failed to parse first part");
    let b = parts[1].parse::<u32>().expect("failed to parse second part");
    NodeId([a, b])
}



impl std::ops::Index<&NodeId> for EGraph {
    type Output = Node;

    fn index(&self, index: &NodeId) -> &Self::Output {
        self.nodes
            .get(index)
            .unwrap_or_else(|| panic!("No node with id {:?}", index))
    }
}

impl std::ops::Index<&ClassId> for EGraph {
    type Output = Class;

    fn index(&self, index: &ClassId) -> &Self::Output {
        self.classes()
            .get(index)
            .unwrap_or_else(|| panic!("No class with id {:?}", index))
    }
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Node_old {
    pub op: String,
    pub id: NodeId_old,
    #[cfg_attr(feature = "serde", serde(default))]
    pub children: Vec<ClassId>,
    pub eclass: ClassId,
    #[cfg_attr(feature = "serde", serde(default = "one"))]
    pub cost: Cost,
}


#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Node {
    pub op: String,
    pub id: NodeId,
    #[cfg_attr(feature = "serde", serde(default))]
    pub children: Vec<ClassId>,
    pub eclass: ClassId,
    #[cfg_attr(feature = "serde", serde(default = "one"))]
    pub cost: Cost,
}

impl Node {
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

fn one() -> Cost {
    Cost::new(1.0).unwrap()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Class {
    pub id: ClassId,
    pub nodes: Vec<NodeId>,
}

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassData {
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub typ: Option<String>,
}
