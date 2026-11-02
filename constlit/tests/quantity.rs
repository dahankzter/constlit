use constlit::k8s::{self, Quantity};

fn hash(q: &Quantity) -> u64 {
    use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
    BuildHasherDefault::<DefaultHasher>::default().hash_one(q)
}

#[test]
fn quantity_accessors() {
    const Q: Quantity = k8s::quantity!("1.5Gi");
    assert_eq!(Q.millis(), 1_610_612_736_000);
    assert_eq!(Q.value(), 1_610_612_736);
    assert_eq!(Q.to_string(), "1.5Gi");
    assert_eq!(Q.as_str(), "1.5Gi");
    assert_eq!(k8s::quantity!("1Ki"), k8s::quantity!("1024"));
    assert_eq!(hash(&k8s::quantity!("1Ki")), hash(&k8s::quantity!("1024")));
    assert_eq!(k8s::quantity!("1Ki").to_string(), "1Ki");
    assert_eq!(k8s::quantity!("-1.5").value(), -2);
    assert_eq!(k8s::quantity!("0.5").value(), 1);
    assert_eq!(k8s::quantity!("1m").value(), 1);
    assert_eq!(k8s::quantity!("-1m").value(), -1);
    assert_eq!(k8s::quantity!("7Ei").value(), 7 << 60);
    assert!(k8s::quantity!("500m") < k8s::quantity!("1"));
    assert_eq!(k8s::cpu!("250m").millis(), 250);
    assert_eq!(k8s::mem!("512Mi").value(), 536_870_912);
    static S: Quantity = k8s::mem!("1Gi");
    assert_eq!(S.value(), 1 << 30);
}

#[cfg(feature = "k8s-openapi")]
#[test]
fn quantity_k8s_openapi() {
    use k8s_openapi::apimachinery::pkg::api::resource::Quantity as K8sQuantity;
    assert_eq!(K8sQuantity::from(k8s::mem!("1.5Gi")).0, "1.5Gi");
    let request: K8sQuantity = k8s::cpu!("250m").into();
    assert_eq!(request, K8sQuantity("250m".into()));
}
