use serde::{
  de::{self, Visitor},
  Deserialize, Deserializer, Serialize,
};
use serde_json::Value;
use smallvec::SmallVec;
use std::{collections::HashMap, fmt, io::Write};

//
// --- Transformer (raw buffer) ---
//
pub struct KvTransformer(pub SmallVec<[u8; 128]>);

impl KvTransformer {
  pub fn with_capacity(cap: usize) -> Self {
    KvTransformer(SmallVec::with_capacity(cap))
  }

  pub fn into_inner(self) -> SmallVec<[u8; 128]> {
    self.0
  }

  pub fn as_slice(&self) -> &[u8] {
    &self.0
  }
}

impl Write for KvTransformer {
  fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
    self.0.extend_from_slice(buf);
    Ok(buf.len())
  }

  fn flush(&mut self) -> std::io::Result<()> {
    Ok(())
  }
}

//
// --- Encoder ---
//
pub struct KvEncoder;

impl KvEncoder {
  /// Encode any serializable value into a KvTransformer
  pub fn encode<T>(value: &T) -> Result<KvTransformer, serde_json::Error>
  where
    T: Serialize + 'static,
  {
    let mut transformer = KvTransformer::with_capacity(128);
    serde_json::to_writer(&mut transformer, &IntOrSer(value))?;
    Ok(transformer)
  }

  /// Encode to pretty JSON
  pub fn encode_pretty<T>(value: &T) -> Result<KvTransformer, serde_json::Error>
  where
    T: Serialize + 'static,
  {
    let mut transformer = KvTransformer::with_capacity(128);
    serde_json::to_writer_pretty(&mut transformer, &IntOrSer(value))?;
    Ok(transformer)
  }
}

//
// --- Decoder ---
//
pub struct KvDecoder;

impl KvDecoder {
  /// Decode to serde_json::Value
  pub fn decode(data: &KvTransformer) -> Result<Value, serde_json::Error> {
    serde_json::from_slice(&data.0)
  }

  /// Decode to a pretty-printed JSON string
  pub fn decode_pretty(data: &KvTransformer) -> Result<String, serde_json::Error> {
    let value: Value = Self::decode(data)?;
    serde_json::to_string_pretty(&value)
  }

  /// Extract specific key from the KV data
  pub fn get_value(data: &KvTransformer, key: &str) -> Option<Value> {
    let parsed: Value = Self::decode(data).ok()?;
    parsed.get(key).cloned()
  }

  /// Get all keys from the KV data
  pub fn get_keys(data: &KvTransformer) -> Vec<String> {
    if let Ok(Value::Object(map)) = Self::decode(data) {
      map.keys().cloned().collect()
    } else {
      Vec::new()
    }
  }

  /// Convert to HashMap<String, String>
  pub fn to_hashmap(data: &KvTransformer) -> HashMap<String, String> {
    let mut map = HashMap::new();

    if let Ok(Value::Object(obj)) = Self::decode(data) {
      for (k, v) in obj {
        let value_str = match v {
          Value::String(s) => s,
          Value::Number(n) => n.to_string(),
          Value::Bool(b) => b.to_string(),
          _ => serde_json::to_string(&v).unwrap_or_default(),
        };
        map.insert(k, value_str);
      }
    }

    map
  }
}

pub struct IntOrSer<'a, T>(pub &'a T);

impl<'a, T> Serialize for IntOrSer<'a, T>
where
  T: Serialize + 'static,
{
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    // handle i64
    if let Some(i) = (self.0 as &dyn std::any::Any).downcast_ref::<i64>() {
      let mut buf = itoa::Buffer::new();
      return serializer.serialize_str(buf.format(*i));
    }

    // handle u64
    if let Some(u) = (self.0 as &dyn std::any::Any).downcast_ref::<u64>() {
      let mut buf = itoa::Buffer::new();
      return serializer.serialize_str(buf.format(*u));
    }

    // handle f64
    if let Some(f) = (self.0 as &dyn std::any::Any).downcast_ref::<f64>() {
      let mut buf = ryu::Buffer::new();
      return serializer.serialize_str(buf.format(*f));
    }

    // handle f32
    if let Some(f) = (self.0 as &dyn std::any::Any).downcast_ref::<f32>() {
      let mut buf = ryu::Buffer::new();
      return serializer.serialize_str(buf.format(*f));
    }

    // fallback
    self.0.serialize(serializer)
  }
}

//
// --- Custom deserializer that parses numbers stored as strings ---
//
pub struct IntOrDe<T>(pub T);

impl<'de, T> Deserialize<'de> for IntOrDe<T>
where
  T: std::str::FromStr,
  <T as std::str::FromStr>::Err: fmt::Display,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct StrOrNumVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T> Visitor<'de> for StrOrNumVisitor<T>
    where
      T: std::str::FromStr,
      <T as std::str::FromStr>::Err: fmt::Display,
    {
      type Value = IntOrDe<T>;

      fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "a number or string containing a number")
      }

      fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        v.parse::<T>()
          .map(IntOrDe)
          .map_err(|e| E::custom(format!("failed to parse '{}': {}", v, e)))
      }

      fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        self.visit_str(&v)
      }

      fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        Ok(IntOrDe(v.to_string().parse::<T>().map_err(|e| {
          E::custom(format!("failed to parse '{}': {}", v, e))
        })?))
      }

      fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        Ok(IntOrDe(v.to_string().parse::<T>().map_err(|e| {
          E::custom(format!("failed to parse '{}': {}", v, e))
        })?))
      }

      fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
      where
        E: de::Error,
      {
        Ok(IntOrDe(v.to_string().parse::<T>().map_err(|e| {
          E::custom(format!("failed to parse '{}': {}", v, e))
        })?))
      }
    }

    deserializer.deserialize_any(StrOrNumVisitor(std::marker::PhantomData))
  }
}
