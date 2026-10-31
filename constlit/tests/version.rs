use constlit::{Version, version};

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

#[test]
fn version_in_const() {
    const V: Version = version!("1.2.3-rc.1");
    assert_eq!(V.pre(), "rc.1");
    assert_eq!((V.major(), V.minor(), V.patch(), V.build()), (1, 2, 3, ""));
    assert!(V > version!("1.2.3-beta.9"));
    assert!(V < version!("1.2.3"));
}

/// A random well-formed SemVer string, biased so that pairs often share a prefix.
fn well_formed(state: &mut u64) -> String {
    const WORDS: [&str; 6] = ["alpha", "beta", "rc", "x-1", "-", "a0"];
    let mut s = format!(
        "{}.{}.{}",
        splitmix(state) % 2,
        splitmix(state) % 2,
        splitmix(state) % 2
    );
    let identifiers = |state: &mut u64| {
        (0..1 + splitmix(state) % 3)
            .map(|_| match splitmix(state) % 4 {
                0 => WORDS[(splitmix(state) % 6) as usize].to_string(),
                1 => (splitmix(state) % 12).to_string(),
                2 => splitmix(state).to_string(),
                _ => format!("{}{}", splitmix(state), splitmix(state)),
            })
            .collect::<Vec<_>>()
            .join(".")
    };
    if !splitmix(state).is_multiple_of(4) {
        s = format!("{s}-{}", identifiers(state));
    }
    if splitmix(state).is_multiple_of(3) {
        s = format!("{s}+{}", identifiers(state));
    }
    s
}

fn ours(v: &semver::Version) -> Version {
    let leak = |s: &str| -> &'static str { Box::leak(s.to_string().into_boxed_str()) };
    Version::__new_unchecked(
        v.major,
        v.minor,
        v.patch,
        leak(v.pre.as_str()),
        leak(v.build.as_str()),
    )
}

#[test]
fn precedence_oracle() {
    let mut state = 3u64;
    for _ in 0..20_000 {
        let a = semver::Version::parse(&well_formed(&mut state)).unwrap();
        let b = semver::Version::parse(&well_formed(&mut state)).unwrap();
        assert_eq!(ours(&a).cmp(&ours(&b)), a.cmp_precedence(&b), "{a} vs {b}");
        assert_eq!(
            ours(&a) == ours(&b),
            a.cmp_precedence(&b).is_eq(),
            "{a} vs {b}"
        );
    }
}

#[cfg(feature = "semver")]
#[test]
fn to_semver() {
    assert_eq!(
        version!("1.2.3-rc.1+b").to_semver(),
        semver::Version::parse("1.2.3-rc.1+b").unwrap()
    );
}
