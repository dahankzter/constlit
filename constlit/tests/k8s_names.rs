use constlit::k8s;

#[test]
fn names_const() {
    const NS: &str = k8s::dns_label!("kube-system");
    const SVC: &str = k8s::dns1035_label!("web");
    const CM: &str = k8s::dns_subdomain!("app.config.v1");
    const KEY: &str = k8s::label_key!("app.kubernetes.io/name");
    const VALUE: &str = k8s::label_value!("Web_App-1");
    assert_eq!(NS, "kube-system");
    assert_eq!(SVC, "web");
    assert_eq!(CM, "app.config.v1");
    assert_eq!(KEY, "app.kubernetes.io/name");
    assert_eq!(VALUE, "Web_App-1");
    assert_eq!(k8s::label_value!(""), "");
    static S: &str = k8s::dns_label!("default");
    assert_eq!(S, "default");
}

#[test]
fn selector_const() {
    const S: &str = k8s::selector!("app=web,env in (prod,staging)");
    assert_eq!(S, "app=web,env in (prod,staging)");
}
