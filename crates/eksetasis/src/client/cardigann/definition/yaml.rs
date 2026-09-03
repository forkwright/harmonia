//! YAML adapter types for the Cardigann definition schema: scalar
//! normalization and author-order-preserving mappings.
//!
//! These exist because serde's derive maps YAML onto Rust's ordered/sorted
//! containers, losing two properties definitions rely on: scalars written as
//! `42` vs `"42"` must compare equal downstream, and mapping order (search
//! fields, `case:` branches) carries semantics.

use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

use super::FieldBlock;

/// A YAML scalar (string, number, or bool) normalized to its string form.
///
/// WHY: definition authors write `id: 42` and `id: "42"` interchangeably;
/// downstream code only ever compares/joins string forms.
///
/// This is also the RAW template representation: the schema structs are
/// generic over `ScalarString` (as deserialized) vs
/// [`ParsedTemplate`](crate::client::cardigann::template::ParsedTemplate)
/// (parsed once at load — see `definition::compile_templates`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScalarString(pub String);

impl AsRef<str> for ScalarString {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<String> for ScalarString {
    fn from(value: String) -> Self {
        ScalarString(value)
    }
}

impl<'de> Deserialize<'de> for ScalarString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ScalarVisitor;

        impl Visitor<'_> for ScalarVisitor {
            type Value = ScalarString;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a YAML scalar (string, number, or bool)")
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(ScalarString(v.to_string()))
            }
        }

        deserializer.deserialize_any(ScalarVisitor)
    }
}

/// A YAML mapping with author order preserved.
#[derive(Debug, Clone)]
pub struct OrderedPairs<T>(pub Vec<(String, T)>);

impl<T> Default for OrderedPairs<T> {
    fn default() -> Self {
        OrderedPairs(Vec::new())
    }
}

impl<'de, T> Deserialize<'de> for OrderedPairs<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PairsVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for PairsVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = OrderedPairs<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(entry) = map.next_entry::<String, T>()? {
                    out.push(entry);
                }
                Ok(OrderedPairs(out))
            }
        }

        deserializer.deserialize_map(PairsVisitor(PhantomData))
    }
}

/// Filter arguments: YAML allows a bare scalar or a list of scalars.
#[derive(Debug, Clone)]
pub struct FilterArgs<T>(pub Vec<T>);

impl<'de, T> Deserialize<'de> for FilterArgs<T>
where
    T: Deserialize<'de> + From<String>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ArgsVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for ArgsVisitor<T>
        where
            T: Deserialize<'de> + From<String>,
        {
            type Value = FilterArgs<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a scalar or a list of scalars")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut out = Vec::new();
                while let Some(item) = seq.next_element::<T>()? {
                    out.push(item);
                }
                Ok(FilterArgs(out))
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![T::from(v.to_string())]))
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![T::from(v.to_string())]))
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![T::from(v.to_string())]))
            }

            fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![T::from(v.to_string())]))
            }

            fn visit_bool<E: de::Error>(self, v: bool) -> Result<Self::Value, E> {
                Ok(FilterArgs(vec![T::from(v.to_string())]))
            }
        }

        deserializer.deserialize_any(ArgsVisitor(PhantomData))
    }
}

/// Search fields in YAML declaration order.
///
/// WHY: `.Result.<field>` templates read the fields extracted EARLIER in the
/// definition, so extraction must follow declaration order; a map's sorted
/// iteration would feed them the wrong subset (and `title:` falling back to
/// `title_default:` is the single most common `.Result` pattern).
#[derive(Debug, Clone)]
pub struct OrderedFields<T = ScalarString>(pub Vec<(String, FieldBlock<T>)>);

impl<T> Default for OrderedFields<T> {
    fn default() -> Self {
        OrderedFields(Vec::new())
    }
}

impl<T> OrderedFields<T> {
    /// Time: O(n) in the number of declared fields (definitions carry a
    /// dozen-odd fields; a map would out-allocate the scan). Space: O(1).
    pub fn get(&self, name: &str) -> Option<&FieldBlock<T>> {
        self.0
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, field)| field)
    }

    /// Time: O(n), Space: O(1) — see [`OrderedFields::get`].
    pub fn contains_key(&self, name: &str) -> bool {
        self.get(name).is_some()
    }

    /// Time: O(1) to hand out the iterator, Space: O(1).
    pub fn iter(&self) -> impl Iterator<Item = &(String, FieldBlock<T>)> {
        self.0.iter()
    }

    /// Time: O(1), Space: O(1).
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Time: O(1), Space: O(1).
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Time: O(n), Space: O(n) — the names are cloned for the row-scope
    /// parser, which outlives the definition borrow.
    pub fn names(&self) -> Vec<String> {
        self.0.iter().map(|(name, _)| name.clone()).collect()
    }
}

impl<'de, T> Deserialize<'de> for OrderedFields<T>
where
    T: Deserialize<'de> + From<String>,
{
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FieldsVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for FieldsVisitor<T>
        where
            T: Deserialize<'de> + From<String>,
        {
            type Value = OrderedFields<T>;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a mapping of field name to field block")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut out: Vec<(String, FieldBlock<T>)> = Vec::new();
                while let Some((name, block)) = map.next_entry::<String, FieldBlock<T>>()? {
                    // WHY: a repeated key overrides in place (last wins,
                    // keeping its first position) rather than duplicating.
                    match out.iter_mut().find(|entry| entry.0 == name) {
                        Some(slot) => slot.1 = block,
                        None => out.push((name, block)),
                    }
                }
                Ok(OrderedFields(out))
            }
        }

        deserializer.deserialize_map(FieldsVisitor(PhantomData))
    }
}
