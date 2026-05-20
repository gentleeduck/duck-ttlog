#[cfg(test)]
mod __test__ {
  use crate::snapshots::{bounded_lz4_decompress, MAX_DECOMPRESSED_SIZE};

  /// Craft a 1 GB-claimed payload: the lz4 size prefix is little-endian u32.
  /// We don't need a valid lz4 body — the bound check must trip first.
  #[test]
  fn rejects_oversized_decompressed_claim() {
    let one_gb: u32 = 1024 * 1024 * 1024;
    let mut payload = one_gb.to_le_bytes().to_vec();
    // Append some bogus "compressed" bytes so the buffer isn't suspiciously empty.
    payload.extend_from_slice(&[0u8; 16]);

    let res = bounded_lz4_decompress(&payload);
    assert!(res.is_err(), "expected error for >256MB decompressed claim");
    let msg = format!("{}", res.unwrap_err());
    assert!(
      msg.contains("exceeds limit"),
      "expected limit-violation error, got: {}",
      msg
    );
  }

  #[test]
  fn rejects_truncated_payload() {
    let payload = [0u8, 0u8, 0u8]; // < 4 bytes, no size prefix
    let res = bounded_lz4_decompress(&payload);
    assert!(res.is_err());
  }

  #[test]
  fn accepts_payload_within_bound() {
    // Compress a small payload with size prepended and confirm we round-trip.
    let original = b"hello ttlog snapshot world".to_vec();
    let compressed =
      lz4::block::compress(&original, Some(lz4::block::CompressionMode::DEFAULT), true)
        .expect("compress");
    let out = bounded_lz4_decompress(&compressed).expect("decompress should succeed");
    assert_eq!(out, original);
  }

  #[test]
  fn bound_is_sane() {
    // Guardrail: an accidental future bump above 4 GB would overflow u32 logic.
    assert!(MAX_DECOMPRESSED_SIZE < u32::MAX as usize);
  }
}
