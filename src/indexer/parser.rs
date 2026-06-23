//! Tree-sitter based multi-language AST parser.

use std::collections::HashMap;
use std::sync::OnceLock;

use tree_sitter::{Language, Parser, Query, QueryCursor, StreamingIterator, Tree};

use crate::error::{FvaError, Result};

/// Supported programming languages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LanguageId {
    Rust,
    Python,
    JavaScript,
    TypeScript,
    Tsx,
    Go,
    C,
    Cpp,
    Java,
    Unknown,
}

impl LanguageId {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" => Self::Rust,
            "py" | "pyi" => Self::Python,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "ts" => Self::TypeScript,
            "tsx" => Self::Tsx,
            "go" => Self::Go,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Self::Cpp,
            "java" => Self::Java,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Tsx => "tsx",
            Self::Go => "go",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Java => "java",
            Self::Unknown => "unknown",
        }
    }

    fn tree_sitter_language(&self) -> Option<Language> {
        match self {
            Self::Rust => Some(tree_sitter_rust::LANGUAGE.into()),
            Self::Python => Some(tree_sitter_python::LANGUAGE.into()),
            Self::JavaScript => Some(tree_sitter_javascript::LANGUAGE.into()),
            Self::TypeScript => Some(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()),
            Self::Tsx => Some(tree_sitter_typescript::LANGUAGE_TSX.into()),
            Self::Go => Some(tree_sitter_go::LANGUAGE.into()),
            Self::C => Some(tree_sitter_c::LANGUAGE.into()),
            Self::Cpp => Some(tree_sitter_cpp::LANGUAGE.into()),
            Self::Java => Some(tree_sitter_java::LANGUAGE.into()),
            Self::Unknown => None,
        }
    }

    /// Tree-sitter query for extracting semantic chunks.
    fn chunk_query(&self) -> Option<&'static str> {
        match self {
            Self::Rust => Some(
                r#"
                (function_item
                  name: (identifier) @name) @chunk

                (impl_item
                  type: (type_identifier) @name) @chunk

                (struct_item
                  name: (type_identifier) @name) @chunk

                (enum_item
                  name: (type_identifier) @name) @chunk

                (trait_item
                  name: (type_identifier) @name) @chunk

                (mod_item
                  name: (identifier) @name) @chunk
                "#,
            ),
            Self::Python => Some(
                r#"
                (function_definition
                  name: (identifier) @name) @chunk

                (class_definition
                  name: (identifier) @name) @chunk
                "#,
            ),
            Self::JavaScript | Self::TypeScript | Self::Tsx => Some(
                r#"
                (function_declaration
                  name: (identifier) @name) @chunk

                (class_declaration
                  name: (identifier) @name) @chunk

                (method_definition
                  name: (property_identifier) @name) @chunk

                (arrow_function) @chunk

                (export_statement
                  declaration: (function_declaration
                    name: (identifier) @name) @chunk)

                (export_statement
                  declaration: (class_declaration
                    name: (identifier) @name) @chunk)
                "#,
            ),
            Self::Go => Some(
                r#"
                (function_declaration
                  name: (identifier) @name) @chunk

                (method_declaration
                  name: (field_identifier) @name) @chunk

                (type_declaration
                  (type_spec
                    name: (type_identifier) @name)) @chunk
                "#,
            ),
            Self::C | Self::Cpp => Some(
                r#"
                (function_definition
                  declarator: (function_declarator
                    declarator: (identifier) @name)) @chunk

                (class_specifier
                  name: (type_identifier) @name) @chunk

                (struct_specifier
                  name: (type_identifier) @name) @chunk
                "#,
            ),
            Self::Java => Some(
                r#"
                (method_declaration
                  name: (identifier) @name) @chunk

                (class_declaration
                  name: (identifier) @name) @chunk

                (interface_declaration
                  name: (identifier) @name) @chunk
                "#,
            ),
            Self::Unknown => None,
        }
    }
}

/// A parsed source file with its AST tree.
pub struct ParsedFile {
    pub language: LanguageId,
    pub tree: Tree,
    pub source: String,
}

/// Multi-language Tree-sitter parser with cached queries.
pub struct AstParser {
    parsers: HashMap<LanguageId, Parser>,
    queries: HashMap<LanguageId, Query>,
}

impl AstParser {
    pub fn new() -> Self {
        let mut parsers = HashMap::new();
        let mut queries = HashMap::new();

        for lang in [
            LanguageId::Rust,
            LanguageId::Python,
            LanguageId::JavaScript,
            LanguageId::TypeScript,
            LanguageId::Tsx,
            LanguageId::Go,
            LanguageId::C,
            LanguageId::Cpp,
            LanguageId::Java,
        ] {
            if let Some(ts_lang) = lang.tree_sitter_language() {
                let mut parser = Parser::new();
                if parser.set_language(&ts_lang).is_ok() {
                    parsers.insert(lang, parser);
                }
            }
            if let Some(query_src) = lang.chunk_query() {
                if let Some(ts_lang) = lang.tree_sitter_language() {
                    if let Ok(query) = Query::new(&ts_lang, query_src) {
                        queries.insert(lang, query);
                    }
                }
            }
        }

        Self { parsers, queries }
    }

    pub fn detect_language(path: &std::path::Path) -> LanguageId {
        path.extension()
            .and_then(|e| e.to_str())
            .map(LanguageId::from_extension)
            .unwrap_or(LanguageId::Unknown)
    }

    pub fn parse(&mut self, language: LanguageId, source: &str) -> Result<Option<ParsedFile>> {
        if language == LanguageId::Unknown {
            return Ok(None);
        }

        let parser = self.parsers.get_mut(&language).ok_or_else(|| {
            FvaError::Parser(format!("unsupported language: {}", language.as_str()))
        })?;

        let tree = parser
            .parse(source, None)
            .ok_or_else(|| FvaError::Parser("tree-sitter parse failed".into()))?;

        Ok(Some(ParsedFile {
            language,
            tree,
            source: source.to_string(),
        }))
    }

    pub fn extract_chunks(&self, parsed: &ParsedFile) -> Vec<RawChunk> {
        let Some(query) = self.queries.get(&parsed.language) else {
            return fallback_chunks(&parsed.source);
        };

        let mut cursor = QueryCursor::new();
        let mut chunks = Vec::new();
        let mut seen_ranges: Vec<(usize, usize)> = Vec::new();

        let mut matches = cursor.matches(query, parsed.tree.root_node(), parsed.source.as_bytes());
        while let Some(m) = matches.next() {
            let mut name = String::new();
            let mut chunk_node = None;

            for capture in m.captures {
                let capture_name = query.capture_names()[capture.index as usize];
                let node = capture.node;
                match capture_name {
                    "name" => {
                        name = node
                            .utf8_text(parsed.source.as_bytes())
                            .unwrap_or("")
                            .to_string();
                    }
                    "chunk" => {
                        chunk_node = Some(node);
                    }
                    _ => {}
                }
            }

            if let Some(node) = chunk_node {
                let start = node.start_byte();
                let end = node.end_byte();
                if is_overlapping(&seen_ranges, start, end) {
                    continue;
                }
                seen_ranges.push((start, end));

                let kind = node.kind().to_string();
                let start_line = node.start_position().row + 1;
                let end_line = node.end_position().row + 1;
                let content = parsed.source[start..end].to_string();

                chunks.push(RawChunk {
                    name: if name.is_empty() {
                        format!("{kind}@L{start_line}")
                    } else {
                        name
                    },
                    kind,
                    start_line,
                    end_line,
                    start_byte: start,
                    end_byte: end,
                    content,
                });
            }
        }

        if chunks.is_empty() {
            return fallback_chunks(&parsed.source);
        }

        chunks.sort_by_key(|c| c.start_line);
        chunks
    }
}

impl Default for AstParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Raw chunk extracted from AST before metadata enrichment.
#[derive(Debug, Clone)]
pub struct RawChunk {
    pub name: String,
    pub kind: String,
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
    pub content: String,
}

fn is_overlapping(ranges: &[(usize, usize)], start: usize, end: usize) -> bool {
    ranges.iter().any(|(s, e)| start < *e && end > *s)
}

/// Fallback: split file into line-based chunks when AST extraction fails.
fn fallback_chunks(source: &str) -> Vec<RawChunk> {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return vec![];
    }

    const CHUNK_LINES: usize = 80;
    let mut chunks = Vec::new();
    let mut start = 0;

    while start < lines.len() {
        let end = (start + CHUNK_LINES).min(lines.len());
        let content = lines[start..end].join("\n");
        chunks.push(RawChunk {
            name: format!("block@L{}", start + 1),
            kind: "block".to_string(),
            start_line: start + 1,
            end_line: end,
            start_byte: 0,
            end_byte: content.len(),
            content,
        });
        start = end;
    }

    chunks
}

static SUPPORTED_EXTENSIONS: OnceLock<Vec<&'static str>> = OnceLock::new();

pub fn supported_extensions() -> &'static [&'static str] {
    SUPPORTED_EXTENSIONS.get_or_init(|| {
        vec![
            "rs", "py", "pyi", "js", "mjs", "cjs", "ts", "tsx", "go", "c", "h", "cpp", "cc", "cxx",
            "hpp", "hxx", "java",
        ]
    })
}

pub fn is_indexable(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| supported_extensions().contains(&ext))
        .unwrap_or(false)
}
