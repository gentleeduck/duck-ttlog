#[cfg(test)]
mod __test__ {

  use std::collections::HashMap;
  use std::io::Write;

  use crate::kv::{IntOrDe, IntOrSer, KvDecoder, KvEncoder, KvTransformer};

  // ── KvTransformer ──────────────────────────────────────────────────

  #[test]
  fn transformer_with_capacity() {
    let t = KvTransformer::with_capacity(64);
    assert!(t.as_slice().is_empty());
  }

  #[test]
  fn transformer_write_and_read() {
    let mut t = KvTransformer::with_capacity(32);
    t.write_all(b"hello").unwrap();
    assert_eq!(t.as_slice(), b"hello");
  }

  #[test]
  fn transformer_into_inner() {
    let mut t = KvTransformer::with_capacity(8);
    t.write_all(b"abc").unwrap();
    let inner = t.into_inner();
    assert_eq!(inner.as_slice(), b"abc");
  }

  #[test]
  fn transformer_flush_succeeds() {
    let mut t = KvTransformer::with_capacity(8);
    assert!(t.flush().is_ok());
  }

  #[test]
  fn transformer_multiple_writes() {
    let mut t = KvTransformer::with_capacity(32);
    t.write_all(b"foo").unwrap();
    t.write_all(b"bar").unwrap();
    assert_eq!(t.as_slice(), b"foobar");
  }

  // ── KvEncoder ──────────────────────────────────────────────────────

  #[test]
  fn encode_string() {
    let val = "hello";
    let t = KvEncoder::encode(&val).unwrap();
    let decoded: serde_json::Value = serde_json::from_slice(t.as_slice()).unwrap();
    assert_eq!(decoded, serde_json::json!("hello"));
  }

  #[test]
  fn encode_map() {
    let mut map = HashMap::new();
    map.insert("key", "value");
    let t = KvEncoder::encode(&map).unwrap();
    let decoded: serde_json::Value = serde_json::from_slice(t.as_slice()).unwrap();
    assert_eq!(decoded["key"], serde_json::json!("value"));
  }

  #[test]
  fn encode_i64_as_string() {
    let val: i64 = 42;
    let t = KvEncoder::encode(&val).unwrap();
    let raw = std::str::from_utf8(t.as_slice()).unwrap();
    // IntOrSer serializes i64 as a string
    assert_eq!(raw, "\"42\"");
  }

  #[test]
  fn encode_u64_as_string() {
    let val: u64 = 999;
    let t = KvEncoder::encode(&val).unwrap();
    let raw = std::str::from_utf8(t.as_slice()).unwrap();
    assert_eq!(raw, "\"999\"");
  }

  #[test]
  fn encode_f64_as_string() {
    let val: f64 = 3.14;
    let t = KvEncoder::encode(&val).unwrap();
    let raw = std::str::from_utf8(t.as_slice()).unwrap();
    // ryu formats floats, wrapped in quotes
    assert!(raw.starts_with('"'));
    assert!(raw.ends_with('"'));
    let inner: f64 = raw.trim_matches('"').parse().unwrap();
    assert!((inner - 3.14).abs() < 1e-10);
  }

  #[test]
  fn encode_f32_as_string() {
    let val: f32 = 2.5;
    let t = KvEncoder::encode(&val).unwrap();
    let raw = std::str::from_utf8(t.as_slice()).unwrap();
    assert!(raw.starts_with('"'));
    let inner: f32 = raw.trim_matches('"').parse().unwrap();
    assert!((inner - 2.5).abs() < 1e-5);
  }

  #[test]
  fn encode_pretty_produces_indented_json() {
    let mut map = HashMap::new();
    map.insert("a", 1);
    let t = KvEncoder::encode_pretty(&map).unwrap();
    let raw = std::str::from_utf8(t.as_slice()).unwrap();
    assert!(raw.contains('\n'));
  }

  #[test]
  fn encode_bool_fallback() {
    let val = true;
    let t = KvEncoder::encode(&val).unwrap();
    let decoded: serde_json::Value = serde_json::from_slice(t.as_slice()).unwrap();
    assert_eq!(decoded, serde_json::json!(true));
  }

  // ── KvDecoder ──────────────────────────────────────────────────────

  #[test]
  fn decode_valid_json() {
    let mut t = KvTransformer::with_capacity(64);
    t.write_all(b"{\"a\":1,\"b\":\"two\"}").unwrap();
    let val = KvDecoder::decode(&t).unwrap();
    assert_eq!(val["a"], serde_json::json!(1));
    assert_eq!(val["b"], serde_json::json!("two"));
  }

  #[test]
  fn decode_pretty_formats_output() {
    let mut t = KvTransformer::with_capacity(64);
    t.write_all(b"{\"x\":10}").unwrap();
    let pretty = KvDecoder::decode_pretty(&t).unwrap();
    assert!(pretty.contains('\n'));
    assert!(pretty.contains("\"x\""));
  }

  #[test]
  fn get_value_existing_key() {
    let mut t = KvTransformer::with_capacity(64);
    t.write_all(b"{\"foo\":\"bar\"}").unwrap();
    let val = KvDecoder::get_value(&t, "foo").unwrap();
    assert_eq!(val, serde_json::json!("bar"));
  }

  #[test]
  fn get_value_missing_key() {
    let mut t = KvTransformer::with_capacity(64);
    t.write_all(b"{\"foo\":1}").unwrap();
    assert!(KvDecoder::get_value(&t, "missing").is_none());
  }

  #[test]
  fn get_keys_returns_all_keys() {
    let mut t = KvTransformer::with_capacity(64);
    t.write_all(b"{\"a\":1,\"b\":2,\"c\":3}").unwrap();
    let mut keys = KvDecoder::get_keys(&t);
    keys.sort();
    assert_eq!(keys, vec!["a", "b", "c"]);
  }

  #[test]
  fn get_keys_non_object_returns_empty() {
    let mut t = KvTransformer::with_capacity(64);
    t.write_all(b"[1,2,3]").unwrap();
    assert!(KvDecoder::get_keys(&t).is_empty());
  }

  #[test]
  fn to_hashmap_basic() {
    let mut t = KvTransformer::with_capacity(128);
    t.write_all(b"{\"name\":\"alice\",\"age\":30,\"active\":true}")
      .unwrap();
    let map = KvDecoder::to_hashmap(&t);
    assert_eq!(map.get("name").unwrap(), "alice");
    assert_eq!(map.get("age").unwrap(), "30");
    assert_eq!(map.get("active").unwrap(), "true");
  }

  #[test]
  fn to_hashmap_with_nested_value() {
    let mut t = KvTransformer::with_capacity(128);
    t.write_all(b"{\"nested\":{\"inner\":1}}").unwrap();
    let map = KvDecoder::to_hashmap(&t);
    let nested = map.get("nested").unwrap();
    // nested objects get serialized back to JSON strings
    assert!(nested.contains("inner"));
  }

  #[test]
  fn to_hashmap_non_object_returns_empty() {
    let mut t = KvTransformer::with_capacity(32);
    t.write_all(b"\"just a string\"").unwrap();
    assert!(KvDecoder::to_hashmap(&t).is_empty());
  }

  #[test]
  fn decode_invalid_json_returns_err() {
    let mut t = KvTransformer::with_capacity(32);
    t.write_all(b"not json").unwrap();
    assert!(KvDecoder::decode(&t).is_err());
  }

  // ── IntOrSer ───────────────────────────────────────────────────────

  #[test]
  fn int_or_ser_i32_uses_fallback() {
    // i32 is NOT i64, so it should use normal serialize (as number, not string)
    let val: i32 = 7;
    let json = serde_json::to_string(&IntOrSer(&val)).unwrap();
    assert_eq!(json, "7");
  }

  #[test]
  fn int_or_ser_negative_i64() {
    let val: i64 = -100;
    let json = serde_json::to_string(&IntOrSer(&val)).unwrap();
    assert_eq!(json, "\"-100\"");
  }

  #[test]
  fn int_or_ser_string_passthrough() {
    let val = "hello world";
    let json = serde_json::to_string(&IntOrSer(&val)).unwrap();
    assert_eq!(json, "\"hello world\"");
  }

  // ── IntOrDe ────────────────────────────────────────────────────────

  #[test]
  fn int_or_de_from_string() {
    let json = "\"42\"";
    let result: IntOrDe<i64> = serde_json::from_str(json).unwrap();
    assert_eq!(result.0, 42);
  }

  #[test]
  fn int_or_de_from_number() {
    let json = "42";
    let result: IntOrDe<i64> = serde_json::from_str(json).unwrap();
    assert_eq!(result.0, 42);
  }

  #[test]
  fn int_or_de_from_float_string() {
    let json = "\"3.14\"";
    let result: IntOrDe<f64> = serde_json::from_str(json).unwrap();
    assert!((result.0 - 3.14).abs() < 1e-10);
  }

  #[test]
  fn int_or_de_from_float_number() {
    let json = "3.14";
    let result: IntOrDe<f64> = serde_json::from_str(json).unwrap();
    assert!((result.0 - 3.14).abs() < 1e-10);
  }

  #[test]
  fn int_or_de_u64_from_string() {
    let json = "\"999\"";
    let result: IntOrDe<u64> = serde_json::from_str(json).unwrap();
    assert_eq!(result.0, 999);
  }

  #[test]
  fn int_or_de_invalid_string_returns_err() {
    let json = "\"not_a_number\"";
    let result = serde_json::from_str::<IntOrDe<i64>>(json);
    assert!(result.is_err());
  }

  // ── Encode → Decode round-trip ─────────────────────────────────────

  #[test]
  fn roundtrip_encode_decode() {
    let mut map = HashMap::new();
    map.insert("status", "ok");
    map.insert("version", "1.0");

    let encoded = KvEncoder::encode(&map).unwrap();
    let decoded = KvDecoder::decode(&encoded).unwrap();

    assert_eq!(decoded["status"], serde_json::json!("ok"));
    assert_eq!(decoded["version"], serde_json::json!("1.0"));
  }

  #[test]
  fn roundtrip_encode_get_keys() {
    let mut map = HashMap::new();
    map.insert("x", 10);
    map.insert("y", 20);

    let encoded = KvEncoder::encode(&map).unwrap();
    let mut keys = KvDecoder::get_keys(&encoded);
    keys.sort();
    assert_eq!(keys, vec!["x", "y"]);
  }

  #[test]
  fn roundtrip_encode_to_hashmap() {
    let mut map = HashMap::new();
    map.insert("key1", "val1");

    let encoded = KvEncoder::encode(&map).unwrap();
    let hm = KvDecoder::to_hashmap(&encoded);
    assert_eq!(hm.get("key1").unwrap(), "val1");
  }

  #[test]
  fn empty_object_roundtrip() {
    let map: HashMap<String, String> = HashMap::new();
    let encoded = KvEncoder::encode(&map).unwrap();
    let decoded = KvDecoder::decode(&encoded).unwrap();
    assert!(decoded.as_object().unwrap().is_empty());
  }
}
