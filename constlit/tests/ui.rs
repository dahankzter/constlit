#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
    // Named time zones depend on the `tz` feature.
    #[cfg(feature = "tz")]
    t.compile_fail("tests/ui_tz/*.rs");
    #[cfg(not(feature = "tz"))]
    t.compile_fail("tests/ui_no_tz/*.rs");
    // Which signals exist depends on the target; these expect generic Linux.
    #[cfg(all(
        target_os = "linux",
        not(any(
            target_arch = "mips",
            target_arch = "mips64",
            target_arch = "mips32r6",
            target_arch = "mips64r6",
            target_arch = "sparc",
            target_arch = "sparc64"
        ))
    ))]
    t.compile_fail("tests/ui_linux/*.rs");
}
