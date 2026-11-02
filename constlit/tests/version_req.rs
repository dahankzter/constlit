use constlit::{Op, Version, VersionReq, version_req};

fn hash<T: std::hash::Hash>(x: &T) -> u64 {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    BuildHasherDefault::<DefaultHasher>::default().hash_one(x)
}

fn splitmix(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Each requirement built by version_req! and by the semver crate.
macro_rules! reqs {
    ($($lit:literal),* $(,)?) => {
        [$((version_req!($lit), semver::VersionReq::parse($lit).unwrap())),*]
    };
}

#[test]
fn version_req_matches() {
    let reqs = reqs![
        "1",
        "1.2",
        "1.2.3",
        "^1",
        "^1.2",
        "^1.2.3",
        "^0",
        "^0.2",
        "^0.2.3",
        "^0.0",
        "^0.0.3",
        "~1",
        "~1.2",
        "~1.2.3",
        "~0.2.3",
        "=1",
        "=1.2",
        "=1.2.3",
        "=1.2.3-alpha",
        ">1",
        ">1.2",
        ">1.2.3",
        ">=1.2.3",
        ">=1.2.3-alpha",
        "<2",
        "<1.2",
        "<1.2.3",
        "<=1.2.3",
        "<1.2.3-beta",
        "1.*",
        "1.2.*",
        "1.x",
        "*",
        ">=1.2, <1.5",
        ">= 1.2 , < 2",
        "^1.2.3-alpha.1",
        "~1.2.3-rc.1",
        ">1.*",
        "=1.*",
        "<=2.x",
        ">=0.0.0-0",
    ];
    let pres = ["", "", "", "alpha", "alpha.1", "beta", "rc.1", "0"];
    let mut state = 9u64;
    for _ in 0..2000 {
        let (major, minor, patch) = (
            splitmix(&mut state) % 4,
            splitmix(&mut state) % 4,
            splitmix(&mut state) % 4,
        );
        let pre = pres[(splitmix(&mut state) % pres.len() as u64) as usize];
        let ours = Version::__new_unchecked(major, minor, patch, pre, "");
        let theirs = semver::Version {
            major,
            minor,
            patch,
            pre: semver::Prerelease::new(pre).unwrap(),
            build: semver::BuildMetadata::EMPTY,
        };
        for (req, sreq) in &reqs {
            assert_eq!(
                req.matches(&ours),
                sreq.matches(&theirs),
                "{req} against {theirs}"
            );
        }
    }
}

#[test]
fn version_req_focus() {
    let v = |s: &'static str| {
        let sv = semver::Version::parse(s).unwrap();
        let pre: &'static str = Box::leak(sv.pre.as_str().to_string().into_boxed_str());
        Version::__new_unchecked(sv.major, sv.minor, sv.patch, pre, "")
    };
    assert!(!version_req!(">=1.2.3").matches(&v("1.3.0-alpha")));
    assert!(version_req!(">=1.2.3-alpha").matches(&v("1.2.3-beta")));
    assert!(!version_req!(">=1.2.3-alpha").matches(&v("1.2.4-alpha")));
    assert!(version_req!("^0.2.3").matches(&v("0.2.4")));
    assert!(!version_req!("^0.2.3").matches(&v("0.3.0")));
    assert!(version_req!("^0.0.3").matches(&v("0.0.3")));
    assert!(!version_req!("^0.0.3").matches(&v("0.0.4")));
    assert!(version_req!("^0").matches(&v("0.9.9")));
}

#[test]
fn version_req_accessors() {
    const R: VersionReq = version_req!(">=1.2, <2");
    assert_eq!(R.comparators().len(), 2);
    assert_eq!(R.comparators()[0].op(), Op::GreaterEq);
    assert_eq!(R.comparators()[0].minor(), Some(2));
    assert_eq!(R.comparators()[1].minor(), None);
    assert_eq!(R.to_string(), ">=1.2, <2");
    assert_eq!(R.as_str(), ">=1.2, <2");
    assert_eq!(version_req!(">=1.2"), version_req!(">= 1.2"));
    assert_eq!(hash(&version_req!(">=1.2")), hash(&version_req!(">= 1.2")));
    assert_ne!(
        version_req!(">=1.2").to_string(),
        version_req!(">= 1.2").to_string()
    );
    assert!(version_req!("*").comparators().is_empty());
    assert_eq!(version_req!("=1.2.3-rc.1").comparators()[0].pre(), "rc.1");
    static S: VersionReq = version_req!("^1");
    assert!(S.matches(&Version::new(1, 9, 0)));
}

#[cfg(feature = "semver")]
#[test]
fn version_req_to_semver() {
    const R: VersionReq = version_req!(">=1.2, <2");
    assert_eq!(
        R.to_semver(),
        semver::VersionReq::parse(">=1.2, <2").unwrap()
    );
    assert_eq!(version_req!("*").to_semver(), semver::VersionReq::STAR);
    assert_eq!(
        version_req!("~1.2.3-beta").to_semver(),
        semver::VersionReq::parse("~1.2.3-beta").unwrap()
    );
}
