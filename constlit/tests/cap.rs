use constlit::linux::{self, CapSet, Capability};

#[test]
fn cap_accessors() {
    const C: Capability = linux::cap!("NET_BIND_SERVICE");
    assert_eq!(C.number(), 10);
    assert_eq!(C.name(), "CAP_NET_BIND_SERVICE");
    assert_eq!(C.to_string(), "CAP_NET_BIND_SERVICE");
    assert_eq!(linux::cap!("CAP_NET_BIND_SERVICE"), C);
    assert_eq!(Capability::from_number(10), Some(C));
    assert_eq!(
        Capability::from_number(40).unwrap().name(),
        "CAP_CHECKPOINT_RESTORE"
    );
    assert_eq!(Capability::from_number(41), None);
    assert!(linux::cap!("CAP_CHOWN") < C);
}

#[test]
fn capset_ops() {
    const S: CapSet = linux::caps!("CAP_NET_ADMIN, CAP_SYS_TIME");
    const C: Capability = linux::cap!("NET_BIND_SERVICE");
    assert_eq!(S.len(), 2);
    assert!(!S.is_empty());
    assert!(S.contains(linux::cap!("CAP_NET_ADMIN")));
    assert!(!S.contains(linux::cap!("CAP_KILL")));
    assert_eq!(S.bits(), (1 << 12) | (1 << 25));
    assert_eq!(S.to_string(), "CAP_NET_ADMIN, CAP_SYS_TIME");
    assert_eq!(S.iter().map(|c| c.number()).collect::<Vec<_>>(), [12, 25]);
    assert_eq!(S.into_iter().count(), 2);
    assert!(CapSet::EMPTY.with(C).without(C).is_empty());
    assert!(CapSet::EMPTY.with(C).contains(C));
    assert_eq!(S.union(CapSet::EMPTY), S);
    assert_eq!(S.union(CapSet::EMPTY.with(C)).len(), 3);
    assert_eq!(S.intersection(CapSet::EMPTY), CapSet::EMPTY);
    assert_eq!(S.intersection(S), S);
    assert!(S.difference(S).is_empty());
    assert_eq!(CapSet::from_bits(1 << 40).map(CapSet::len), Some(1));
    assert_eq!(CapSet::from_bits(1 << 41), None);
    assert_eq!(S.iter().collect::<CapSet>(), S);
    assert_eq!(CapSet::EMPTY.to_string(), "");
    assert_eq!(
        linux::caps!("CAP_CHECKPOINT_RESTORE").iter().next(),
        Capability::from_number(40)
    );

    static ST: CapSet = linux::caps!("KILL");
    assert_eq!(ST.len(), 1);
}

#[cfg(target_os = "linux")]
#[test]
fn cap_table_oracle() {
    let all = caps::all();
    assert_eq!(all.len(), 41);
    for c in all {
        let ours = Capability::from_number(c.index()).unwrap();
        assert_eq!(ours.name(), c.to_string());
    }
}
