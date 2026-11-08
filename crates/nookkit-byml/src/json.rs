use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::ser::{self, Serialize, SerializeMap, SerializeSeq, Serializer};

use crate::byml::Node;
use crate::error::Invalid;

// Plain JSON with no type tags, as Heaven Tool's `BymlJson` writes it. An integer that fits 32 bits
// is an Int and a bigger one an Int64, a number with a decimal point or an exponent is a Float.
// Nodes that wouldn't read back the same (UInt, UInt64, Binary and an Int64 that fits 32 bits)
// aren't written
impl Serialize for Node {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let unsupported = |kind| Err(ser::Error::custom(Invalid::Unsupported(kind)));

        match self {
            Node::Dict(dict) => {
                let mut map = s.serialize_map(Some(dict.len()))?;

                for (key, value) in dict {
                    map.serialize_entry(key, value)?;
                }

                map.end()
            }
            Node::Array(items) => {
                let mut seq = s.serialize_seq(Some(items.len()))?;

                for item in items {
                    seq.serialize_element(item)?;
                }

                seq.end()
            }
            Node::String(text) => s.serialize_str(text),
            Node::Bool(value) => s.serialize_bool(*value),
            Node::Int(value) => s.serialize_i32(*value),
            Node::Int64(value) if i32::try_from(*value).is_err() => s.serialize_i64(*value),
            // serde_json writes a float with a decimal point or an exponent, 30.0 and not 30
            Node::Float(value) if value.is_finite() => s.serialize_f32(*value),
            Node::Float(value) => Err(ser::Error::custom(Invalid::NotFinite(*value))),
            Node::Null => s.serialize_unit(),
            Node::Int64(_) => unsupported("Int64 that fits in 32 bits"),
            Node::UInt(_) => unsupported("UInt"),
            Node::UInt64(_) => unsupported("UInt64"),
            Node::Binary(_) => unsupported("Binary"),
        }
    }
}

impl<'de> Deserialize<'de> for Node {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_any(NodeVisitor)
    }
}

struct NodeVisitor;

impl<'de> Visitor<'de> for NodeVisitor {
    type Value = Node;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a BYML node as plain JSON")
    }

    fn visit_unit<E: de::Error>(self) -> Result<Node, E> {
        Ok(Node::Null)
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Node, E> {
        Ok(Node::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Node, E> {
        Ok(i32::try_from(value).map_or(Node::Int64(value), Node::Int))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Node, E> {
        let value = i64::try_from(value).map_err(|_| E::custom(Invalid::Integer(value)))?;

        self.visit_i64(value)
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Node, E> {
        Ok(Node::Float(value as f32))
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Node, E> {
        Ok(Node::String(text.to_owned()))
    }

    fn visit_string<E: de::Error>(self, text: String) -> Result<Node, E> {
        Ok(Node::String(text))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Node, A::Error> {
        let mut items = Vec::new();

        while let Some(item) = seq.next_element()? {
            items.push(item);
        }

        Ok(Node::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Node, A::Error> {
        let mut dict = BTreeMap::new();

        while let Some(key) = map.next_key::<String>()? {
            if dict.contains_key(&key) {
                return Err(de::Error::custom(Invalid::DuplicateKey(key)));
            }

            let value = map.next_value()?;
            dict.insert(key, value);
        }

        Ok(Node::Dict(dict))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(json: &str) -> Node {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn tells_numbers_apart_by_their_text() {
        assert_eq!(node("30"), Node::Int(30));
        assert_eq!(node("-2147483648"), Node::Int(i32::MIN));
        assert_eq!(node("2147483648"), Node::Int64(2_147_483_648));
        assert_eq!(node("30.0"), Node::Float(30.0));
        assert_eq!(node("1e3"), Node::Float(1000.0));
        assert!(serde_json::from_str::<Node>("9223372036854775808").is_err());
    }

    #[test]
    fn writes_floats_with_a_point() {
        let json = serde_json::to_string(&Node::Array(vec![
            Node::Float(30.0),
            Node::Float(0.1),
            Node::Float(1e20),
            Node::Int(30),
            Node::Int64(1 << 40),
        ]))
        .unwrap();

        assert_eq!(json, "[30.0,0.1,1e+20,30,1099511627776]");
    }

    #[test]
    fn reads_dictionaries_and_refuses_repeated_keys() {
        let Node::Dict(dict) = node(r#"{"b": [true, null, "x"], "a": {}}"#) else {
            panic!("not a dictionary");
        };

        assert_eq!(dict.keys().collect::<Vec<_>>(), ["a", "b"]);

        assert_eq!(
            dict["b"],
            Node::Array(vec![Node::Bool(true), Node::Null, Node::String("x".into())])
        );

        assert!(serde_json::from_str::<Node>(r#"{"a": 1, "a": 2}"#).is_err());
    }

    #[test]
    fn refuses_nodes_that_wouldnt_read_back() {
        for value in [
            Node::UInt(1),
            Node::UInt64(1),
            Node::Int64(1),
            Node::Binary(vec![]),
            Node::Float(f32::NAN),
        ] {
            assert!(serde_json::to_string(&value).is_err(), "{value:?}");
        }
    }
}
