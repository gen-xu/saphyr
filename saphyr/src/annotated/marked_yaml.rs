//! A YAML node with position in the source document.
//!
//! This is set aside so as to not clutter `annotated.rs`.

use std::path::Path;

use hashlink::LinkedHashMap;
use saphyr_parser::{BufferedInput, Input, Parser, ScanError, Span};

use crate::{LoadableYamlNode, Yaml, YamlData, YamlLoader};

#[derive(Debug)]
pub enum LoadError {
    Scan(ScanError),
    Io(std::io::Error),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for LoadError {}

impl From<ScanError> for LoadError {
    fn from(value: ScanError) -> Self {
        Self::Scan(value)
    }
}

impl From<std::io::Error> for LoadError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

/// A YAML node with [`Span`]s pointing to the start of the node.
///
/// This structure does not implement functions to operate on the YAML object. To access those,
/// refer to the [`Self::data`] field.
#[derive(Clone, Debug)]
pub struct MarkedYaml {
    /// The span indicating where in the input stream the object is.
    ///
    /// The markers are relative to the start of the input stream that was given to the parser, not
    /// to the start of the document within the input stream.
    pub span: Span,
    /// The YAML contents of the node.
    pub data: YamlData<MarkedYaml>,
}

impl MarkedYaml {
    /// Load the given string as an array of YAML documents.
    ///
    /// See the function [`load_from_str`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_str`]: `Yaml::load_from_str`
    pub fn load_from_str(source: &str) -> Result<Vec<Self>, ScanError> {
        Self::load_from_iter(source.chars())
    }
    /// Load the given string as an array of YAML documents.
    ///
    /// See the function [`load_from_str`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_str`]: `Yaml::load_from_str`
    pub fn load_from_file(path_path: impl AsRef<Path>) -> Result<Vec<Self>, LoadError> {
        let file_path = path_path.as_ref();
        let source = std::fs::read_to_string(file_path)?;
        let mut parser = Parser::new(BufferedInput::new(
            source.chars(),
            Some(file_path.to_path_buf()),
        ));
        Ok(Self::load_from_parser(&mut parser)?)
    }

    /// Load the contents of the given iterator as an array of YAML documents.
    ///
    /// See the function [`load_from_str`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_str`]: `Yaml::load_from_str`
    #[inline(always)]
    pub fn load_from_iter<I: Iterator<Item = char>>(source: I) -> Result<Vec<Self>, ScanError> {
        let mut parser = Parser::new(BufferedInput::new(source, None));
        Self::load_from_parser(&mut parser)
    }

    /// Load the contents from the specified [`Parser`] as an array of YAML documents.
    ///
    /// See the function [`load_from_str`] for more details.
    ///
    /// # Errors
    /// Returns `ScanError` when loading fails.
    ///
    /// [`load_from_str`]: `Yaml::load_from_str`
    #[inline(always)]
    pub fn load_from_parser<I: Input>(parser: &mut Parser<I>) -> Result<Vec<Self>, ScanError> {
        let mut loader = YamlLoader::<Self>::default();
        parser.load(&mut loader, true)?;
        Ok(loader.into_documents())
    }
}

impl PartialEq for MarkedYaml {
    fn eq(&self, other: &Self) -> bool {
        self.data.eq(&other.data)
    }
}

// I don't know if it's okay to implement that, but we need it for the hashmap.
impl Eq for MarkedYaml {}

impl std::hash::Hash for MarkedYaml {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.data.hash(state);
    }
}

impl From<YamlData<MarkedYaml>> for MarkedYaml {
    fn from(value: YamlData<MarkedYaml>) -> Self {
        Self {
            span: Span::default(),
            data: value,
        }
    }
}

impl LoadableYamlNode for MarkedYaml {
    fn from_bare_yaml(yaml: Yaml) -> Self {
        Self {
            span: Span::default(),
            data: match yaml {
                Yaml::Real { value, tag } => YamlData::Real { value, tag },
                Yaml::Integer { value, tag } => YamlData::Integer { value, tag },
                Yaml::String { value, tag } => YamlData::String { value, tag },
                Yaml::Boolean { value, tag } => YamlData::Bool { value, tag },
                // Array and Hash will always have their container empty.
                Yaml::Sequence { value: _, tag } => YamlData::Sequence { value: vec![], tag },
                Yaml::Map { value: _, tag } => YamlData::Map {
                    value: LinkedHashMap::new(),
                    tag,
                },
                Yaml::Alias(x) => YamlData::Alias(x),
                Yaml::Null => YamlData::Null,
                Yaml::BadValue => YamlData::BadValue,
            },
        }
    }

    fn is_sequence(&self) -> bool {
        self.data.is_sequence()
    }

    fn display(&self) -> String {
        format!("{self:?}")
    }
    fn is_merge_key(&self) -> bool {
        self.data.as_str().is_some_and(|s| s.starts_with("<<"))
    }

    fn is_map(&self) -> bool {
        self.data.is_map()
    }

    fn is_badvalue(&self) -> bool {
        self.data.is_badvalue()
    }

    fn sequence_mut(&mut self) -> &mut Vec<Self> {
        if let YamlData::Sequence { value, .. } = &mut self.data {
            value
        } else {
            panic!("Called array_mut on a non-array");
        }
    }

    fn into_map(self) -> LinkedHashMap<Self, Self> {
        if let YamlData::Map { value, .. } = self.data {
            value
        } else {
            panic!("Called into_map on a non-map");
        }
    }

    fn map_mut(&mut self) -> &mut LinkedHashMap<Self, Self> {
        if let YamlData::Map { value, .. } = &mut self.data {
            value
        } else {
            panic!("Called map_mut on a non-map");
        }
    }

    fn take(&mut self) -> Self {
        let mut taken_out = MarkedYaml {
            span: Span::default(),
            data: YamlData::BadValue,
        };
        std::mem::swap(&mut taken_out, self);
        taken_out
    }

    fn with_span(mut self, span: Span) -> Self {
        self.span = span;
        self
    }
}
