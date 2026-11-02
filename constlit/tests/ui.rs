#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
    // Named time zones depend on the `tz` feature.
    #[cfg(feature = "tz")]
    t.compile_fail("tests/ui_tz/*.rs");
    #[cfg(not(feature = "tz"))]
    t.compile_fail("tests/ui_no_tz/*.rs");
}
