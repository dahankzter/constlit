use constlit::oci::{self, Digest, DigestAlgorithm};

#[test]
fn digest_accessors() {
    const D: Digest =
        oci::digest!("sha256:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20");
    assert_eq!(D.algorithm(), DigestAlgorithm::Sha256);
    assert_eq!(D.as_bytes(), &(1u8..=32).collect::<Vec<_>>()[..]);
    assert_eq!(
        D.to_string(),
        "sha256:0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"
    );
    const L: Digest = oci::digest!(
        "sha512:cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd"
    );
    assert_eq!(L.as_bytes().len(), 64);
    assert_eq!(L.algorithm().name(), "sha512");
    assert_eq!(L.as_bytes()[0], 0xcd);
    assert_ne!(D, L);
    static S: Digest =
        oci::digest!("sha256:0000000000000000000000000000000000000000000000000000000000000000");
    assert_eq!(S.as_bytes(), &[0; 32]);
}

fn hash<T: std::hash::Hash>(x: &T) -> u64 {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    BuildHasherDefault::<DefaultHasher>::default().hash_one(x)
}

#[test]
fn image_accessors() {
    use constlit::oci::ImageRef;
    const I: ImageRef = oci::image!("ghcr.io/org/app:1.2");
    assert_eq!(I.registry(), "ghcr.io");
    assert_eq!(I.repository(), "org/app");
    assert_eq!(I.tag(), Some("1.2"));
    assert_eq!(I.digest(), None);
    assert_eq!(I.canonical(), "ghcr.io/org/app:1.2");
    assert_eq!(I.to_string(), "ghcr.io/org/app:1.2");
    assert_eq!(I.as_str(), "ghcr.io/org/app:1.2");

    let short = oci::image!("nginx");
    let long = oci::image!("docker.io/library/nginx:latest");
    assert_eq!(short, long);
    assert_eq!(hash(&short), hash(&long));
    assert_ne!(short.to_string(), long.to_string());
    assert_eq!(short.canonical(), "docker.io/library/nginx:latest");

    let pinned = oci::image!(
        "nginx@sha256:abababababababababababababababababababababababababababababababab"
    );
    assert_eq!(
        pinned.digest(),
        Some(oci::digest!(
            "sha256:abababababababababababababababababababababababababababababababab"
        ))
    );
    assert_eq!(pinned.tag(), None);

    static S: ImageRef = oci::image!("localhost:5000/app");
    assert_eq!(S.registry(), "localhost:5000");
}

#[test]
fn platform_accessors() {
    use constlit::oci::Platform;
    const P: Platform = oci::platform!("linux/arm64/v8");
    assert_eq!(P.os(), "linux");
    assert_eq!(P.arch(), "arm64");
    assert_eq!(P.variant(), Some("v8"));
    assert_eq!(P.to_string(), "linux/arm64/v8");
    assert_ne!(oci::platform!("linux/arm64"), P);
    assert_eq!(oci::platform!("windows/amd64").variant(), None);
    assert_eq!(oci::platform!("windows/amd64").to_string(), "windows/amd64");
    static S: Platform = oci::platform!("linux/amd64");
    assert_eq!(S.arch(), "amd64");
}
