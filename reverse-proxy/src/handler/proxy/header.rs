use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use serde::{Deserialize, Serialize};

/// An ordered list of HTTP header `(name, value)` pairs.
///
/// Mirrors the interceptor's `L8Headers`. Duplicate names are allowed (e.g. multiple
/// `Set-Cookie`, `Vary`, `Link` headers) and insertion order is preserved. Name lookups are
/// ASCII case-insensitive; names are stored exactly as provided.
///
/// Wire format: a newtype over `Vec<(String, String)>` encodes exactly like the inner vector
/// (length prefix followed by key/value pairs), which is byte-compatible with the
/// interceptor and with peers still using a `HashMap<String, String>`.
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Default, bincode::Encode, bincode::Decode,
)]
pub struct L8Header(Vec<(String, String)>);

// The binary target compiles this module privately, so parts of the API not used there would
// otherwise warn as dead code.
#[allow(dead_code)]
impl L8Header {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Builds headers from any iterator of `(name, value)` pairs, keeping duplicates and order.
    pub fn from_pairs<K, V>(pairs: impl IntoIterator<Item = (K, V)>) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        Self(
            pairs
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        )
    }

    /// Returns the first value for `name` (case-insensitive).
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    /// Returns all values for `name` (case-insensitive), in order.
    pub fn get_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.0
            .iter()
            .filter(move |(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }

    pub fn contains(&self, name: &str) -> bool {
        self.0.iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
    }

    /// Adds a value, keeping any existing values for the same name.
    pub fn append(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.0.push((name.into(), value.into()));
    }

    /// Sets a value, replacing all existing values for the same name.
    pub fn insert(&mut self, name: impl Into<String>, value: impl Into<String>) {
        let name = name.into();
        self.remove(&name);
        self.0.push((name, value.into()));
    }

    /// Removes all values for `name` (case-insensitive).
    pub fn remove(&mut self, name: &str) {
        self.0.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Read-only iterator over `(name, value)` pairs in order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Merges `other` into `self`. For every name present in `other`, the existing values in
    /// `self` are replaced by all of `other`'s values for that name (duplicates in `other`
    /// are kept).
    pub fn extend(&mut self, other: L8Header) {
        for (name, _) in other.0.iter() {
            self.remove(name);
        }
        self.0.extend(other.0);
    }

    /// Converts into a `HeaderMap`. Duplicate names are appended, so every value is kept.
    /// Fails on the first invalid header name or value.
    pub fn to_headermap(&self) -> Result<HeaderMap, Box<dyn std::error::Error>> {
        let mut headers = HeaderMap::with_capacity(self.0.len());
        for (k, v) in &self.0 {
            let name = HeaderName::from_bytes(k.as_bytes())?;
            let value = HeaderValue::from_str(v)?;
            headers.append(name, value);
        }
        Ok(headers)
    }

    /// Builds from a `HeaderMap`, keeping duplicate names. Values that are not visible ASCII
    /// become empty strings.
    pub fn from_headermap(headers: &HeaderMap) -> Self {
        Self(
            headers
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
                .collect(),
        )
    }
}

impl From<&HeaderMap> for L8Header {
    fn from(headers: &HeaderMap) -> Self {
        Self::from_headermap(headers)
    }
}

impl<K: Into<String>, V: Into<String>> FromIterator<(K, V)> for L8Header {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self::from_pairs(iter)
    }
}

impl IntoIterator for L8Header {
    type Item = (String, String);
    type IntoIter = std::vec::IntoIter<(String, String)>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn lookups_are_case_insensitive_and_keep_duplicates() {
        let mut h = L8Header::from_pairs([("Set-Cookie", "a=1"), ("Content-Type", "text/plain")]);
        h.append("set-cookie", "b=2");

        assert_eq!(h.get("SET-COOKIE"), Some("a=1"));
        assert_eq!(h.get_all("set-cookie").collect::<Vec<_>>(), vec!["a=1", "b=2"]);
        assert!(h.contains("content-type"));
        assert_eq!(h.len(), 3);

        h.insert("set-cookie", "c=3");
        assert_eq!(h.get_all("Set-Cookie").collect::<Vec<_>>(), vec!["c=3"]);

        h.remove("CONTENT-TYPE");
        assert!(!h.contains("content-type"));
    }

    #[test]
    fn headermap_round_trip_keeps_duplicates() {
        let h = L8Header::from_pairs([("set-cookie", "a=1"), ("set-cookie", "b=2"), ("x-a", "1")]);
        let map = h.to_headermap().unwrap();
        assert_eq!(map.get_all("set-cookie").iter().count(), 2);

        let back = L8Header::from_headermap(&map);
        assert_eq!(back.get_all("set-cookie").collect::<Vec<_>>(), vec!["a=1", "b=2"]);
        assert_eq!(back.get("x-a"), Some("1"));
    }

    #[test]
    fn to_headermap_rejects_invalid_name() {
        assert!(L8Header::from_pairs([("bad name", "v")]).to_headermap().is_err());
    }

    #[test]
    fn bincode_wire_format_matches_hashmap() {
        let cfg = bincode::config::standard();
        let h = L8Header::from_pairs([("k", "v")]);
        let m = HashMap::from([("k".to_string(), "v".to_string())]);
        assert_eq!(
            bincode::encode_to_vec(&h, cfg).unwrap(),
            bincode::encode_to_vec(&m, cfg).unwrap()
        );
        let (decoded, _): (L8Header, _) =
            bincode::decode_from_slice(&bincode::encode_to_vec(&m, cfg).unwrap(), cfg).unwrap();
        assert_eq!(decoded, h);
    }
}
