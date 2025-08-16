use std::ops::Add;
use std::str::FromStr;
use serde::de::{Deserialize};

pub fn string_to_number<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Add<Output = T> + Copy + FromStr, <T as FromStr>::Err: std::fmt::Display,
{
    let s: String = Deserialize::deserialize(deserializer).map_err(|e| {
        serde::de::Error::custom(format!("Failed to deserialize string to number: {}", e))
    })?;
    s.parse::<T>().map_err(serde::de::Error::custom)
}

pub fn string_to_u8_32<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).map_err(|e| {
        serde::de::Error::custom(format!("Failed to deserialize string to u8[32]: {}", e))
    })?;
    let bytes = s.into_bytes();
    if bytes.len() != 32 {
        return Err(serde::de::Error::custom("Expected 32 bytes for nTor static secret"));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
}

pub fn string_to_vec_u8<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).map_err(|e| {
        serde::de::Error::custom(format!("Failed to deserialize string to vec<u8>: {}", e))
    })?;
    Ok(s.into_bytes())
}

pub fn string_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer).map_err(|e| {
        serde::de::Error::custom(format!("Failed to deserialize string to bool: {}", e))
    })?;
    match s.to_lowercase().as_str() {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        _ => Err(serde::de::Error::custom("Expected 'true' or 'false'")),
    }
}