use grammar::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fmt;
use tree_fold::TreeFold;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CppType {
    Int,
    Int64T,
    String,
}

impl Default for CppType {
    fn default() -> Self {
        CppType::String
    }
}

impl fmt::Display for CppType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CppType::Int => write!(f, "int"),
            CppType::Int64T => write!(f, "int64_t"),
            CppType::String => write!(f, "std::string"),
        }
    }
}

derive_serialize_from_display!(CppType);

#[derive(Debug, Eq, PartialEq, Serialize)]
pub enum CppCodeBlock {
    NodeEnvoyProperty(String),
    CallUserFunc(String),
    CallLibFunc(String),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct NodeAttribute<'a> {
    pub id: &'a str,
    pub parts: Vec<&'a str>,
    pub value: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct AttributeDef<'a> {
    pub typ: CppType,
    pub parts: Vec<&'a str>,
}

#[derive(Serialize, PartialEq, Eq, Debug, Clone)]
pub enum UdfType {
    Scalar,
    Aggregation,
}

impl Default for UdfType {
    fn default() -> Self {
        UdfType::Scalar
    }
}

#[derive(Default, Serialize, PartialEq, Eq, Debug, Clone)]
pub struct Udf<'a> {
    pub udf_type: UdfType,
    pub id: &'a str,
    pub func_impl: &'a str,
    pub return_type: CppType,
    pub arg: Vec<&'a str>,
}

#[derive(Serialize, PartialEq, Eq, Debug)]
pub enum CppResult {
    Return { typ: CppType, id: String },
    GroupBy { typ: CppType, id: String },
    None,
}

impl<'a> Default for CppResult {
    fn default() -> Self {
        CppResult::None
    }
}

pub struct CodeGenConfig<'a> {
    pub attributes_to_property_parts: HashMap<&'static str, AttributeDef<'a>>,
    pub udf_table: HashMap<&'static str, Udf<'a>>,
}

impl<'a> CodeGenConfig<'a> {
    pub fn new() -> Self {
        Default::default()
    }
}

impl<'a> Default for CodeGenConfig<'a> {
    fn default() -> Self {
        let mut attributes_to_property_parts = HashMap::new();
        attributes_to_property_parts.insert(
            "service_name",
            AttributeDef {
                typ: CppType::String,
                parts: vec!["node", "metadata", "WORKLOAD_NAME"],
            },
        );

        attributes_to_property_parts.insert(
            "response_size",
            AttributeDef {
                typ: CppType::Int64T,
                parts: vec!["response", "total_size"],
            },
        );

        CodeGenConfig {
            attributes_to_property_parts,
            udf_table: HashMap::new(),
        }
    }
}

#[derive(Default, Serialize)]
pub struct CodeGen<'a> {
    // Graph structures
    pub root_id: &'a str,
    pub vertices: HashSet<&'a str>,
    pub edges: Vec<(&'a str, &'a str)>,
    pub nodes_to_attributes: Vec<NodeAttribute<'a>>,

    // Envoy node attribute initializer lists.
    pub node_attributes_to_fetch: HashSet<AttributeDef<'a>>,

    // Intermediate computations necessary for computing result
    pub blocks: Vec<CppCodeBlock>,
    pub udfs: Vec<Udf<'a>>,
    // Final computation result
    pub result: CppResult,

    #[serde(skip_serializing)]
    pub config: CodeGenConfig<'a>,
}

impl<'a> CodeGen<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn new_with_config(config: CodeGenConfig<'a>) -> Self {
        CodeGen {
            config,
            ..Default::default()
        }
    }
}

impl<'a> TreeFold<'a> for CodeGen<'a> {
    fn visit_prog(&mut self, prog: &'a Prog) {
        self.visit_patterns(&prog.patterns);
        self.visit_filters(&prog.filters);
        self.visit_action(&prog.action);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        let src_id = pattern.from_node.id_name;
        let dst_id = pattern.to_node.id_name;

        // TODO: Handle edge ids and edge properties.
        let _edge_id = match &pattern.relationship_type {
            Relationship::Edge(id) => String::from(id.id_name),
            Relationship::Path(_) => panic!("TODO: support EDGE relatipnship type"),
        };

        self.vertices.insert(src_id);
        self.vertices.insert(dst_id);
        self.edges.push((src_id, dst_id));
    }

    fn visit_filter(&mut self, filter: &'a Filter) {
        let Filter::Property(id, p, value) = filter;

        let node_id = id.id_name;

        let attribute_def = &self.config.attributes_to_property_parts[p.id_name];

        self.nodes_to_attributes.push(NodeAttribute {
            id: node_id,
            parts: attribute_def.parts.clone(),
            value: value.to_string(),
        });
        self.node_attributes_to_fetch.insert(attribute_def.clone());
    }

    fn visit_action(&mut self, action: &'a Action) {
        match action {
            Action::GetProperty(id, p) => {
                if id.id_name == "target" {
                    if p.id_name == "height" {
                        let cpp_var_id = "get_tree_height_target";

                        let block = format!(
                            "std::string {cpp_var_id} = std::to_string({func_name}({args}));",
                            cpp_var_id = cpp_var_id,
                            func_name = "get_tree_height",
                            args = "target"
                        );

                        self.blocks.push(CppCodeBlock::CallLibFunc(block));

                        self.result = CppResult::Return {
                            typ: CppType::Int,
                            id: String::from(cpp_var_id),
                        };
                    } else {
                        panic!("{} graph property not supported", p.id_name)
                    }
                } else {
                    let attribute = &self.config.attributes_to_property_parts[p.id_name];

                    let cpp_var_id: String =
                        String::from(id.id_name) + "_" + &attribute.parts.join("_");

                    let mut parts = String::from("{");
                    parts.push_str(
                        &attribute
                            .parts
                            .iter()
                            .map(|s| String::from("\"") + s + "\"")
                            .collect::<Vec<String>>()
                            .join(", "),
                    );
                    parts.push_str("}");

                    let block = format!(
                        "node_ptr = get_node_with_id(target, mapping->at(\"{node_id}\"));
if (node_ptr == nullptr || node_ptr->properties.find({parts}) == node_ptr->properties.end()) {{
    LOG_WARN(\"Node {node_id} not found\");
    return;
}}
std::string {cpp_var_id} = node_ptr->properties.at({parts});",
                        node_id = id.id_name,
                        parts = parts,
                        cpp_var_id = cpp_var_id,
                    );
                    self.blocks.push(CppCodeBlock::NodeEnvoyProperty(block));
                    self.node_attributes_to_fetch.insert(attribute.clone());

                    self.result = CppResult::Return {
                        typ: attribute.typ,
                        id: cpp_var_id,
                    };
                }
            }
            Action::None => {}
            Action::CallUdf(id) => {
                if !self.config.udf_table.contains_key(id.id_name) {
                    panic!("Can't find udf function: {}", id.id_name);
                }
                let func = &self.config.udf_table[id.id_name];

                let cpp_var_id = String::from(func.id) + "_result";

                let block = if func.return_type != CppType::String {
                    format!(
                        "std::string {cpp_var_id} = std::to_string(root_->{func_name}_udf_({args}));",
                        cpp_var_id = cpp_var_id,
                        func_name = func.id,
                        args = "target"
                    )
                } else {
                    format!(
                        "std::string {cpp_var_id} = root_->{func_name}_udf_({args});",
                        cpp_var_id = cpp_var_id,
                        func_name = func.id,
                        args = "target"
                    )
                };

                self.blocks.push(CppCodeBlock::CallUserFunc(block));

                self.udfs.push(func.clone());

                self.result = CppResult::Return {
                    typ: func.return_type,
                    id: cpp_var_id,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer;
    use parser;
    use std::iter::Peekable;
    use token::Token;

    #[test]
    fn test_match() {
        let tokens: Vec<Token> = lexer::get_tokens(r"MATCH a-->b : x,b-->c : y,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree: Prog = parser::parse_prog(&mut token_iter);

        let mut code_gen = CodeGen::new();
        code_gen.visit_prog(&parse_tree);

        assert_eq!(code_gen.vertices, ["a", "b", "c"].iter().cloned().collect());
        assert_eq!(code_gen.edges, vec![("a", "b"), ("b", "c")]);
        assert!(code_gen.nodes_to_attributes.is_empty());
    }

    #[test]
    fn test_match_handle_ordering() {
        let tokens: Vec<Token> = lexer::get_tokens(r"MATCH b-->c : x,a-->b : y,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree: Prog = parser::parse_prog(&mut token_iter);

        let mut code_gen = CodeGen::new();
        code_gen.visit_prog(&parse_tree);

        assert_eq!(code_gen.vertices, ["a", "b", "c"].iter().cloned().collect());
        assert_eq!(code_gen.edges, vec![("b", "c"), ("a", "b")]);
        assert!(code_gen.nodes_to_attributes.is_empty());
    }

    #[test]
    fn test_codegen_parallel() {
        let tokens: Vec<Token> = lexer::get_tokens(r"MATCH b-->c : x,a-->b : y,a-->d:z,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree: Prog = parser::parse_prog(&mut token_iter);

        let mut code_gen = CodeGen::new();
        code_gen.visit_prog(&parse_tree);

        assert_eq!(
            code_gen.vertices,
            ["a", "b", "c", "d"].iter().cloned().collect()
        );
        assert_eq!(code_gen.edges, vec![("b", "c"), ("a", "b"), ("a", "d")]);
        assert!(code_gen.nodes_to_attributes.is_empty());
    }

    #[test]
    fn test_codegen_action() {
        let tokens: Vec<Token> = lexer::get_tokens(r"MATCH n-->m: a, WHERE n.x == k, RETURN n.x,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree = parser::parse_prog(&mut token_iter);
        let mut code_gen = CodeGen::new_with_config(CodeGenConfig {
            attributes_to_property_parts: [(
                "x",
                AttributeDef {
                    typ: CppType::String,
                    parts: vec!["x"],
                },
            )]
            .iter()
            .cloned()
            .collect(),
            ..Default::default()
        });
        code_gen.visit_prog(&parse_tree);

        assert_eq!(code_gen.vertices, ["n", "m"].iter().cloned().collect());
        assert_eq!(code_gen.edges, vec![("n", "m")]);
        assert_eq!(
            code_gen.blocks,
            vec![CppCodeBlock::NodeEnvoyProperty(String::from(
                "node_ptr = get_node_with_id(target, mapping->at(\"n\"));
if (node_ptr == nullptr || node_ptr->properties.find({\"x\"}) == node_ptr->properties.end()) {
    LOG_WARN(\"Node n not found\");
    return;
}
std::string n_x = node_ptr->properties.at({\"x\"});"
            ))]
        );
        assert_eq!(
            code_gen.nodes_to_attributes,
            vec![NodeAttribute {
                id: "n",
                parts: vec!["x"],
                value: String::from("k"),
            }]
        );
        assert_eq!(
            code_gen.result,
            CppResult::Return {
                typ: CppType::String,
                id: String::from("n_x"),
            }
        )
    }

    #[test]
    fn test_codegen_action_without_where() {
        let tokens: Vec<Token> = lexer::get_tokens(r"MATCH n-->m: a, RETURN n.x,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree = parser::parse_prog(&mut token_iter);
        let mut code_gen = CodeGen::new_with_config(CodeGenConfig {
            attributes_to_property_parts: [(
                "x",
                AttributeDef {
                    typ: CppType::String,
                    parts: vec!["x"],
                },
            )]
            .iter()
            .cloned()
            .collect(),
            ..Default::default()
        });

        code_gen.visit_prog(&parse_tree);

        assert_eq!(code_gen.vertices, ["n", "m"].iter().cloned().collect());
        assert_eq!(code_gen.edges, vec![("n", "m")]);
        assert_eq!(
            code_gen.blocks,
            vec![CppCodeBlock::NodeEnvoyProperty(String::from(
                "node_ptr = get_node_with_id(target, mapping->at(\"n\"));
if (node_ptr == nullptr || node_ptr->properties.find({\"x\"}) == node_ptr->properties.end()) {
    LOG_WARN(\"Node n not found\");
    return;
}
std::string n_x = node_ptr->properties.at({\"x\"});"
            ))]
        );
        assert_eq!(
            code_gen.result,
            CppResult::Return {
                typ: CppType::String,
                id: String::from("n_x"),
            }
        )
    }

    #[test]
    fn test_codegen_multiple_node_properties_to_collect() {
        let tokens: Vec<Token> = lexer::get_tokens(r"MATCH n-->m: a, WHERE n.x ==k, RETURN n.y,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree = parser::parse_prog(&mut token_iter);
        let mut code_gen = CodeGen::new_with_config(CodeGenConfig {
            attributes_to_property_parts: [
                (
                    "x",
                    AttributeDef {
                        typ: CppType::String,
                        parts: vec!["x"],
                    },
                ),
                (
                    "y",
                    AttributeDef {
                        typ: CppType::Int64T,
                        parts: vec!["y"],
                    },
                ),
            ]
            .iter()
            .cloned()
            .collect(),
            ..Default::default()
        });

        code_gen.visit_prog(&parse_tree);

        assert_eq!(code_gen.vertices, ["n", "m"].iter().cloned().collect());
        assert_eq!(code_gen.edges, vec![("n", "m")]);
        assert_eq!(
            code_gen.nodes_to_attributes,
            vec![NodeAttribute {
                id: "n",
                parts: vec!["x"],
                value: String::from("k"),
            }]
        );
        assert_eq!(
            code_gen.node_attributes_to_fetch,
            vec![
                AttributeDef {
                    typ: CppType::String,
                    parts: vec!["x"]
                },
                AttributeDef {
                    typ: CppType::Int64T,
                    parts: vec!["y"]
                }
            ]
            .iter()
            .cloned()
            .collect()
        );
    }

    #[test]
    fn test_codegen_graph_properties() {
        let tokens = lexer::get_tokens(r"MATCH n-->m: a, RETURN target.height,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree = parser::parse_prog(&mut token_iter);
        let mut code_gen = CodeGen::new();
        code_gen.visit_prog(&parse_tree);
        assert_eq!(code_gen.vertices, ["n", "m"].iter().cloned().collect());
        assert_eq!(code_gen.edges, vec![("n", "m")]);
        assert_eq!(
            code_gen.blocks,
            vec![CppCodeBlock::CallLibFunc(String::from(
                "std::string get_tree_height_target = std::to_string(get_tree_height(target));"
            ))]
        );
        assert_eq!(
            code_gen.result,
            CppResult::Return {
                typ: CppType::Int,
                id: String::from("get_tree_height_target")
            }
        );
    }

    #[test]
    fn test_codegen_call_udf() {
        let tokens = lexer::get_tokens(r"MATCH n-->m: a, RETURN max_response_size,");
        let mut token_iter: Peekable<std::slice::Iter<Token>> = tokens.iter().peekable();
        let parse_tree = parser::parse_prog(&mut token_iter);
        let mut code_gen = CodeGen::new();
        code_gen.config.udf_table.insert(
            "max_response_size",
            Udf {
                udf_type: UdfType::Scalar,
                id: "max_response_size",
                func_impl: "function_impl",
                return_type: CppType::Int64T,
                arg: vec![],
            },
        );
        code_gen.visit_prog(&parse_tree);
        assert_eq!(code_gen.vertices, ["n", "m"].iter().cloned().collect());
        assert_eq!(code_gen.edges, vec![("n", "m")]);

        assert_eq!(
            code_gen.blocks,
            vec![CppCodeBlock::CallUserFunc(String::from(
                "std::string max_response_size_result = std::to_string(root_->max_response_size_udf_(target));"
            ))]
        );
        assert_eq!(
            code_gen.udfs,
            vec![Udf {
                udf_type: UdfType::Scalar,
                id: "max_response_size",
                func_impl: "function_impl",
                return_type: CppType::Int64T,
                arg: vec![],
            }],
        );
        assert_eq!(
            code_gen.result,
            CppResult::Return {
                typ: CppType::Int64T,
                id: String::from("max_response_size_result"),
            }
        );
    }
}
