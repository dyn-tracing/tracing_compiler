/***********************************/
// IR Structs
/***********************************/
use std::collections::HashMap;
use serde::{Serialize, Serializer};

#[derive(Clone, Debug, Serialize)]
pub struct StructuralFilter {
    pub vertices: Vec<String>,
    pub edges: Vec<(String, String)>,
    pub properties: HashMap<String, HashMap<String, String>>, // attribute, value
}
impl Default for StructuralFilter {
    fn default() -> Self {
        StructuralFilter {
            vertices: Vec::new(),
            edges: Vec::new(),
            properties: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct AttributeFilter {
    pub node: String,
    pub property: String,
    pub value: String,
}
impl Default for AttributeFilter {
    fn default() -> Self {
        AttributeFilter {
            node: String::new(),
            property: String::new(),
            value: String::new(),
        }
    }
}

impl AttributeFilter {
    pub fn insert_values(&mut self, node: String, property: String, value: String) {
        self.node = node;
        self.property = property;
        self.value = value;
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct IrReturn {
    pub entity: String,
    pub property: String,
}

impl IrReturn {
    pub fn new_with_items(entity: String, property: String) -> Self {
        IrReturn { entity, property }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct VisitorResults {
    pub struct_filters: Vec<StructuralFilter>,
    pub prop_filters: Vec<AttributeFilter>,
    pub return_expr: Option<IrReturn>,
    pub aggregate: Option<Aggregate>,
    pub maps: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Aggregate {
    pub udf_id: String,
    pub entity: String,
    pub property: String,
}
impl Aggregate {
    pub fn new_with_items(entity: String, property: String, udf: String) -> Self {
        Aggregate {
            udf_id: udf,
            entity,
            property,
        }
    }
}
