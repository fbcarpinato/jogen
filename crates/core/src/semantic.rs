use std::path::Path;
use tree_sitter::{Language, Node, Parser, Tree};

pub enum SupportedLanguage {
    Rust,
    JavaScript,
    Python,
}

impl SupportedLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "js" | "ts" | "jsx" | "tsx" => Some(Self::JavaScript),
            "py" => Some(Self::Python),
            _ => None,
        }
    }

    pub fn get_tree_sitter_language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
        }
    }
}

pub struct SemanticBlock {
    pub kind: String,
    pub name: String,
    pub content: String,
    pub start_line: usize,
    pub end_line: usize,
    pub breadcrumbs: Vec<String>,
}

pub struct SemanticEngine;

impl Default for SemanticEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticEngine {
    pub fn new() -> Self {
        Self
    }

    /// Attempts to parse the file content into an AST if the language is supported.
    pub fn parse_file(&self, path: &Path, content: &[u8]) -> Option<(SupportedLanguage, Tree)> {
        let ext = path.extension()?.to_str()?;
        let lang = SupportedLanguage::from_extension(ext)?;

        let mut parser = Parser::new();
        if parser.set_language(&lang.get_tree_sitter_language()).is_err() {
            return None;
        }

        let tree = parser.parse(content, None)?;
        Some((lang, tree))
    }

    /// Extracts semantic blocks recursively from an AST.
    pub fn extract_blocks(&self, tree: &Tree, content: &[u8]) -> Vec<SemanticBlock> {
        let mut blocks = Vec::new();
        self.collect_blocks(tree.root_node(), content, Vec::new(), &mut blocks);
        blocks
    }

    fn collect_blocks(&self, node: Node, content: &[u8], breadcrumbs: Vec<String>, blocks: &mut Vec<SemanticBlock>) {
        let kind = node.kind();
        let is_block = kind.contains("function") 
            || kind.contains("class") 
            || kind.contains("struct") 
            || kind.contains("impl") 
            || kind.contains("declaration")
            || kind.contains("import")
            || kind.contains("export")
            || kind.contains("static")
            || kind.contains("const");

        let mut new_breadcrumbs = breadcrumbs.clone();

        if is_block {
            let name = self.find_identifier(node, content).unwrap_or_else(|| "<unnamed>".to_string());
            let block_content = if let Ok(s) = std::str::from_utf8(&content[node.start_byte()..node.end_byte()]) {
                s.to_string()
            } else {
                "".to_string()
            };

            blocks.push(SemanticBlock {
                kind: kind.to_string(),
                name: name.clone(),
                content: block_content,
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                breadcrumbs: breadcrumbs.clone(),
            });
            new_breadcrumbs.push(name);
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_blocks(child, content, new_breadcrumbs.clone(), blocks);
        }
    }

    /// Recursively searches for the first "identifier" or "name" node to label the block.
    fn find_identifier(&self, node: Node, content: &[u8]) -> Option<String> {
        let kind = node.kind();
        if kind == "identifier" || kind == "name" || kind == "type_identifier" {
            return std::str::from_utf8(&content[node.start_byte()..node.end_byte()])
                .ok()
                .map(|s| s.to_string());
        }

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            // Prefer fields specifically labeled as "name" in tree-sitter grammars
            if let Some(field) = child.walk().field_name() {
                 if field == "name" {
                     if let Ok(s) = std::str::from_utf8(&content[child.start_byte()..child.end_byte()]) {
                         return Some(s.to_string());
                     }
                 }
            }

            if let Some(id) = self.find_identifier(child, content) {
                return Some(id);
            }
        }
        None
    }
}

