use constlit::{ByteSize, size};

#[test]
fn size_in_const() {
    const L: ByteSize = size!("64 KiB");
    assert_eq!(L.bytes(), 65536);
    const M: ByteSize = size!("128Mi");
    assert_eq!(M, ByteSize::from_bytes(128 << 20));
}
