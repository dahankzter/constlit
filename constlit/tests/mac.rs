use constlit::{MacAddr, mac};

#[test]
fn mac_accessors() {
    const M: MacAddr = mac!("02:00:5e:10:00:01");
    assert!(M.is_local());
    assert!(!M.is_universal());
    assert!(M.is_unicast());
    assert!(!M.is_multicast());
    assert!(mac!("01:00:5e:00:00:fb").is_multicast());
    assert!(mac!("00:1a:2b:3c:4d:5e").is_universal());

    let broadcast = mac!("FF:FF:FF:FF:FF:FF");
    assert_eq!(broadcast.to_string(), "ff:ff:ff:ff:ff:ff");
    assert!(broadcast.is_multicast());

    assert_eq!(MacAddr::new(M.octets()), M);
    assert_eq!(<[u8; 6]>::from(M), [0x02, 0x00, 0x5e, 0x10, 0x00, 0x01]);
    assert_eq!(MacAddr::from([0; 6]), mac!("00-00-00-00-00-00"));
    assert!(mac!("00:00:00:00:00:01") < mac!("00:00:00:00:01:00"));

    static S: MacAddr = mac!("00:00:00:00:00:01");
    assert_eq!(S.octets()[5], 1);
}
