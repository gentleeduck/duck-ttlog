//! Bounded LZ4 decompression shared by snapshot read-back utilities.
//!
//! Snapshot `.bin` files are attacker-influenced input (anyone who can write
//! to the snapshot directory). A crafted file can declare an enormous
//! decompressed size and force an unbounded allocation (decompression bomb).
//! [`bounded_lz4_decompress`] reads the size prefix and rejects oversized
//! claims before any allocation happens.

use std::error::Error;

use lz4::block::decompress;

/// Hard upper bound on a decompressed snapshot payload. Anything larger is
/// treated as a decompression bomb and rejected before allocation.
pub const MAX_DECOMPRESSED_SIZE: usize = 256 * 1024 * 1024; // 256 MB

/// Reads the little-endian u32 size prefix that `lz4::block::compress(.., true)`
/// writes and refuses payloads claiming to expand beyond
/// [`MAX_DECOMPRESSED_SIZE`].
pub fn bounded_lz4_decompress(buf: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
  if buf.len() < 4 {
    return Err("lz4 payload missing size prefix".into());
  }
  let claimed = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
  if claimed > MAX_DECOMPRESSED_SIZE {
    return Err(format!(
      "lz4 decompressed size {} exceeds limit {}",
      claimed, MAX_DECOMPRESSED_SIZE
    )
    .into());
  }
  // Pass `None` so the library reads the prepended size itself; we have already
  // bounded the claim above.
  Ok(decompress(buf, None)?)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rejects_missing_prefix() {
    assert!(bounded_lz4_decompress(&[1, 2, 3]).is_err());
  }

  #[test]
  fn rejects_oversized_claim() {
    // Size prefix claiming ~4 GiB of decompressed data.
    let mut buf = u32::MAX.to_le_bytes().to_vec();
    buf.extend_from_slice(&[0u8; 8]);
    let err = bounded_lz4_decompress(&buf).unwrap_err();
    assert!(err.to_string().contains("exceeds limit"));
  }

  #[test]
  fn round_trips_within_bound() {
    let original = b"ttlog snapshot payload that compresses fine";
    let compressed = lz4::block::compress(original, None, true).unwrap();
    let restored = bounded_lz4_decompress(&compressed).unwrap();
    assert_eq!(restored, original);
  }
}
