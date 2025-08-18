#[cfg(test)]
mod tests {
  use crate::string_interner::StringInterner;

  #[test]
  fn test_string_interner() {
    let string_interner = StringInterner::new();
    string_interner.intern_string(
      "wildduck",
      &string_interner.targets,
      &string_interner.target_lookup,
    );

    string_interner.intern_string(
      "ahmedayoub",
      &string_interner.targets,
      &string_interner.target_lookup,
    );
  }
}
