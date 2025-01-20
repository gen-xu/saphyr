//! Utilities for extracting YAML with certain metadata.

pub mod marked_yaml;

use std::ops::{Index, IndexMut};

use hashlink::LinkedHashMap;
use saphyr_parser::Tag;

use crate::loader::parse_f64;

/// YAML data for nodes that will contain annotations.
///
/// If you want a YAML node without annotations, see [`Yaml`].
/// If you want a YAML node with annotations, see types using [`YamlData`] such as [`MarkedYaml`]
///
/// Unlike [`Yaml`] which only supports storing data, [`YamlData`] allows storing metadata
/// alongside the YAML data. It is unlikely one would build it directly; it is mostly intended to
/// be used, for instance, when parsing a YAML where retrieving markers / comments is relevant.
///
/// This definition is recursive. Each annotated node will be a structure storing the annotations
/// and the YAML data. We need to have a distinct enumeration from [`Yaml`] because the type for
/// the `Array` and `Hash` variants is dependant on that structure.
///
/// If we had written [`YamlData`] as:
/// ```ignore
/// pub enum YamlData {
///   // ...
///   Array(Vec<Yaml>),
///   Hash(LinkedHashMap<Yaml, Yaml>),
///   // ...
/// }
/// ```
/// we would have stored metadata for the root node only. All subsequent nodes would be [`Yaml`],
/// which does not contain any annotation.
///
/// Notable differences with [`Yaml`]:
///   * Indexing cannot return `BadValue` and will panic instead.
///
/// [`Yaml`]: crate::Yaml
/// [`MarkedYaml`]: marked_yaml::MarkedYaml
#[derive(Clone, PartialEq, PartialOrd, Debug, Eq, Ord, Hash)]
pub enum YamlData<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    /// Float types are stored as String and parsed on demand.
    /// Note that `f64` does NOT implement Eq trait and can NOT be stored in `BTreeMap`.
    Real {
        /// The value of the YAML node.
        value: Box<str>,
        /// The tag of the YAML node.
        tag: Option<Tag>,
    },
    /// YAML int is stored as i64.
    Integer {
        /// The value of the YAML node.
        value: i64,
        /// The tag of the YAML node.
        tag: Option<Tag>,
    },
    /// YAML scalar.
    String {
        /// The value of the YAML node.
        value: Box<str>,
        /// The tag of the YAML node.
        tag: Option<Tag>,
    },
    /// YAML bool, e.g. `true` or `false`.
    Bool {
        /// The value of the YAML node.
        value: bool,
        /// The tag of the YAML node.
        tag: Option<Tag>,
    },
    /// YAML array, can be accessed as a `Vec`.
    Sequence {
        /// The value of the YAML node.
        value: Vec<Node>,
        /// The tag of the YAML node.
        tag: Option<Tag>,
    },
    /// YAML hash, can be accessed as a `LinkedHashMap`.
    ///
    /// Insertion order will match the order of insertion into the map.
    Map {
        /// The value of the YAML node.
        value: AnnotatedMap<Node>,
        /// The tag of the YAML node.
        tag: Option<Tag>,
    },
    /// Alias, not fully supported yet.
    Alias(usize),
    /// YAML null, e.g. `null` or `~`.
    Null,
    /// Accessing a nonexistent node via the Index trait returns `BadValue`. This
    /// simplifies error handling in the calling code. Invalid type conversion also
    /// returns `BadValue`.
    BadValue,
}

/// The type contained in the [`YamlData::Hash`] variant. This corresponds to YAML mappings.
#[allow(clippy::module_name_repetitions)]
pub type AnnotatedMap<Node> = LinkedHashMap<Node, Node>;

impl<Node> YamlData<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    define_as!(as_bool, bool, Bool);
    define_as!(as_i64, i64, Integer);

    define_as_ref!(as_map, &AnnotatedMap<Node>, Map);
    define_as_ref!(as_str, &str, String);
    define_as_ref!(as_sequence, &Vec<Node>, Sequence);

    define_as_mut_ref!(as_map_mut, &mut AnnotatedMap<Node>, Map);
    define_as_mut_ref!(as_sequence_mut, &mut Vec<Node>, Sequence);

    define_into!(into_bool, bool, Bool);
    define_into!(into_map, AnnotatedMap<Node>, Map);
    define_into!(into_i64, i64, Integer);
    define_into!(into_string, Box<str>, String);
    define_into!(into_sequence, Vec<Node>, Sequence);

    define_is!(is_alias, Self::Alias(_));
    define_is!(is_sequence, Self::Sequence { .. });
    define_is!(is_badvalue, Self::BadValue);
    define_is!(is_boolean, Self::Bool { .. });
    define_is!(is_map, Self::Map { .. });
    define_is!(is_integer, Self::Integer { .. });
    define_is!(is_null, Self::Null);
    define_is!(is_real, Self::Real { .. });
    define_is!(is_string, Self::String { .. });

    /// Get the tag of the YAML node.
    ///
    /// # Return
    /// If the node has a tag, return `Some(&Tag)`. Otherwise, return `None`.
    #[must_use]
    pub fn get_tag(&self) -> Option<&Tag> {
        match self {
            YamlData::Real { tag, .. } => tag.as_ref(),
            YamlData::Integer { tag, .. } => tag.as_ref(),
            YamlData::String { tag, .. } => tag.as_ref(),
            YamlData::Bool { tag, .. } => tag.as_ref(),
            YamlData::Sequence { tag, .. } => tag.as_ref(),
            YamlData::Map { tag, .. } => tag.as_ref(),
            _ => None,
        }
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`YamlData::Real`] YAML node or its contents is not a valid `f64`
    /// string, `None` is returned.
    #[must_use]
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Real { value, .. } = self {
            parse_f64(value)
        } else {
            None
        }
    }

    /// Return the `f64` value contained in this YAML node.
    ///
    /// If the node is not a [`YamlData::Real`] YAML node or its contents is not a valid `f64`
    /// string, `None` is returned.
    #[must_use]
    pub fn into_f64(self) -> Option<f64> {
        self.as_f64()
    }

    /// If a value is null or otherwise bad (see variants), consume it and
    /// replace it with a given value `other`. Otherwise, return self unchanged.
    ///
    /// See [`Yaml::or`] for examples.
    ///
    /// [`Yaml::or`]: crate::Yaml::or
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        match self {
            Self::BadValue | Self::Null => other,
            this => this,
        }
    }

    /// See [`Self::or`] for behavior.
    ///
    /// This performs the same operations, but with borrowed values for less linear pipelines.
    #[must_use]
    pub fn borrowed_or<'a>(&'a self, other: &'a Self) -> &'a Self {
        match self {
            Self::BadValue | Self::Null => other,
            this => this,
        }
    }
}

// NOTE(ethiraric, 10/06/2024): We cannot create a "generic static" variable which would act as a
// `BAD_VALUE`. This means that, unlike for `Yaml`, we have to make the indexing method panic.

impl<'a, Node> Index<&'a str> for YamlData<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    type Output = Node;

    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`YamlData::Hash`].
    fn index(&self, idx: &'a str) -> &Node {
        let key = Self::String {
            value: idx.to_owned().into_boxed_str(),
            tag: None,
        };
        match self.as_map() {
            Some(h) => h.get(&key.into()).unwrap(),
            None => panic!("{idx}: key does not exist"),
        }
    }
}

impl<'a, Node> IndexMut<&'a str> for YamlData<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    /// Perform indexing if `self` is a mapping.
    ///
    /// # Panics
    /// This function panics if the key given does not exist within `self` (as per [`Index`]).
    ///
    /// This function also panics if `self` is not a [`YamlData::Hash`].
    fn index_mut(&mut self, idx: &'a str) -> &mut Node {
        let key = Self::String {
            value: idx.to_owned().into_boxed_str(),
            tag: None,
        };
        match self.as_map_mut() {
            Some(h) => h.get_mut(&key.into()).unwrap(),
            None => panic!("Not a hash type"),
        }
    }
}

impl<Node> Index<usize> for YamlData<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    type Output = Node;

    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`Index`]). If `self` is a
    /// [`YamlData::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`YamlData::Hash`], this is when the mapping sequence does
    /// not contain [`YamlData::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`YamlData::Array`] nor a [`YamlData::Hash`].
    fn index(&self, idx: usize) -> &Node {
        if let Some(v) = self.as_sequence() {
            v.get(idx).unwrap()
        } else if let Some(v) = self.as_map() {
            let key = Self::Integer {
                value: i64::try_from(idx).unwrap(),
                tag: None,
            };
            v.get(&key.into()).unwrap()
        } else {
            panic!("{idx}: Index out of bounds");
        }
    }
}

impl<Node> IndexMut<usize> for YamlData<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    /// Perform indexing if `self` is a sequence or a mapping.
    ///
    /// # Panics
    /// This function panics if the index given is out of range (as per [`IndexMut`]). If `self` is
    /// a [`YamlData::Array`], this is when the index is bigger or equal to the length of the
    /// underlying `Vec`. If `self` is a [`YamlData::Hash`], this is when the mapping sequence does
    /// not contain [`YamlData::Integer`]`(idx)` as a key.
    ///
    /// This function also panics if `self` is not a [`YamlData::Array`] nor a [`YamlData::Hash`].
    fn index_mut(&mut self, idx: usize) -> &mut Node {
        match self {
            Self::Sequence {
                value: sequence, ..
            } => sequence.index_mut(idx),
            Self::Map { value: mapping, .. } => {
                let key = Self::Integer {
                    value: i64::try_from(idx).unwrap(),
                    tag: None,
                };
                mapping.get_mut(&key.into()).unwrap()
            }
            _ => panic!("Attempting to index but `self` is not a sequence nor a mapping"),
        }
    }
}

impl<Node> IntoIterator for YamlData<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<Self>,
{
    type Item = Node;
    type IntoIter = AnnotatedYamlIter<Node>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            yaml: self.into_sequence().unwrap_or_default().into_iter(),
        }
    }
}

/// An iterator over a [`YamlData`] node.
#[allow(clippy::module_name_repetitions)]
pub struct AnnotatedYamlIter<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<Node>>,
{
    yaml: std::vec::IntoIter<Node>,
}

impl<Node> Iterator for AnnotatedYamlIter<Node>
where
    Node: std::hash::Hash + std::cmp::Eq + From<YamlData<Node>>,
{
    type Item = Node;

    fn next(&mut self) -> Option<Node> {
        self.yaml.next()
    }
}
