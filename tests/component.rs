use dekopon_kubectl_provider::Kubectl;
use dekopon_provider_sdk::provider::{Request, Response, method};
use dekopon_provider_sdk::{CommandRunOutcome, provider};
use dekopon_provider_sdk_testkit::{Harness, HttpScript, Native};
use serde_json::json;

const HOST: &str = "kubernetes.example.test";
const CAPABILITY: &str = "kubectl.get.readyz";

fn component() -> std::path::PathBuf {
    std::env::var_os("DEKOPON_PROVIDER_COMPONENT")
        .expect("DEKOPON_PROVIDER_COMPONENT must name the freshly built component")
        .into()
}

fn response(status: u16, body: &[u8]) -> HttpScript {
    HttpScript::new(
        HOST,
        method::GET,
        Response {
            status,
            headers: Vec::new(),
            body: body.to_vec(),
        },
    )
}

fn assert_readyz_request(request: &Request, origin: &str) {
    assert_eq!(request.uri, format!("{origin}/readyz"));
    assert_eq!(request.method, method::GET);
    assert!(request.body.is_empty());
    assert!(
        !request
            .headers
            .iter()
            .any(|header| header.name.eq_ignore_ascii_case("authorization"))
    );
}

#[test]
fn the_component_gets_the_fixed_readiness_endpoint_once() {
    let run = Harness::<Kubectl>::get(component()).http(response(200, b"ok"));
    let origin = run.origin().expect("scripted origin").to_owned();
    let result = run
        .settings(json!({ "baseUrl": origin }))
        .call(CAPABILITY, json!({}))
        .expect("invoke readiness on the component");
    assert_eq!(result.status, 0);
    assert_eq!(result.stdout, b"ok\n");
    assert!(result.stderr.is_empty());
    assert_eq!(result.http_calls.len(), 1);
    assert_readyz_request(&result.http_request.expect("readiness request"), &origin);
}

#[test]
fn native_readiness_command_proposes_the_only_capability() {
    let manifest = provider::manifest::<Kubectl>().expect("valid manifest");
    assert_eq!(manifest.command_words, ["kubectl"]);
    let [capability] = manifest.capabilities.as_slice() else {
        panic!("bootstrap exports exactly one capability");
    };
    assert_eq!(capability.id.as_str(), CAPABILITY);

    let words = ["get", "--raw", "/readyz"].map(str::to_owned);
    let CommandRunOutcome::Proposed {
        capability,
        input,
        secret_use: None,
    } = provider::command::<Kubectl>(&words, false)
    else {
        panic!("readyz proposes exactly one capability");
    };
    assert_eq!(capability.as_str(), CAPABILITY);
    assert_eq!(input, json!({}));

    let origin = format!("https://{HOST}");
    let native = Native::<Kubectl>::new()
        .settings(json!({ "baseUrl": origin }))
        .http(response(200, b"ok"));
    let result = native.call(capability.as_str(), &input.to_string());
    assert_eq!(result.status, 0);
    assert_eq!(result.stdout, b"ok\n");
    let requests = native.requests();
    let [request] = requests.as_slice() else {
        panic!("readyz sends one request");
    };
    assert_readyz_request(request, &origin);
}

#[test]
fn native_command_rejects_unsupported_invocations() {
    let cases: &[&[&str]] = &[
        &[],
        &["get", "secret", "x"],
        &["get", "configmap"],
        &["get", "configmap", "x"],
        &["get", "pods"],
        &["exec"],
        &["exec", "pod-x", "--", "true"],
        &["logs"],
        &["logs", "pod-x"],
        &["--token", "placeholder", "get", "--raw", "/readyz"],
        &["--kubeconfig", "test-file", "get", "--raw", "/readyz"],
        &["get", "--raw", "/api/v1/namespaces/dekopon/secrets"],
        &["get", "--raw", "/livez"],
        &["get", "--raw", "/readyz", "-o", "json"],
        &["get", "--help"],
    ];
    for words in cases {
        let argv: Vec<_> = words.iter().map(|word| (*word).to_owned()).collect();
        let CommandRunOutcome::Rendered {
            stdout,
            stderr,
            status,
        } = provider::command::<Kubectl>(&argv, false)
        else {
            panic!("unsupported words must not propose a capability");
        };
        assert_eq!(status, 2, "{words:?}");
        assert!(stdout.is_empty(), "{words:?}");
        assert!(!stderr.is_empty(), "{words:?}");
    }
}

#[test]
fn the_component_reports_non_success_status_and_malformed_readiness_as_errors() {
    for (status, body, stderr) in [
        (503, &b"ok"[..], "readiness returned HTTP 503\n"),
        (
            200,
            &b"unexpected payload"[..],
            "readiness response did not contain ok\n",
        ),
    ] {
        let run = Harness::<Kubectl>::get(component()).http(response(status, body));
        let origin = run.origin().expect("scripted origin").to_owned();
        let result = run
            .settings(json!({ "baseUrl": origin }))
            .call(CAPABILITY, json!({}))
            .expect("capture readiness failure");
        assert_eq!(result.status, 1);
        assert!(result.stdout.is_empty());
        assert_eq!(result.stderr, stderr);
        assert_eq!(result.http_calls.len(), 1);
        assert_readyz_request(&result.http_request.expect("readiness request"), &origin);
    }
}

#[test]
fn native_transport_failure_reports_an_error_after_one_request() {
    let origin = format!("https://{HOST}");
    let native = Native::<Kubectl>::new().settings(json!({ "baseUrl": origin }));
    let result = native.call(CAPABILITY, "{}");
    assert_eq!(result.status, 1);
    assert!(result.stdout.is_empty());
    assert_eq!(result.stderr, "readiness request failed\n");
    let requests = native.requests();
    let [request] = requests.as_slice() else {
        panic!("readyz attempts one request");
    };
    assert_readyz_request(request, &origin);
}

#[test]
fn native_readiness_rejects_http_and_path_bearing_origins_before_a_request() {
    for origin in [format!("http://{HOST}"), format!("https://{HOST}/proxy")] {
        let native = Native::<Kubectl>::new()
            .settings(json!({ "baseUrl": origin }))
            .http(response(200, b"ok"));
        let result = native.call(CAPABILITY, "{}");
        assert_eq!(result.status, 1, "{origin}");
        assert!(result.stdout.is_empty(), "{origin}");
        assert_eq!(
            result.stderr, "baseUrl must be an HTTPS origin\n",
            "{origin}"
        );
        assert!(native.requests().is_empty(), "{origin}");
    }
}
