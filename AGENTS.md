# Guidance for coding agents

The Kubernetes provider for Dekopon exports one Wasm component and the `kubectl` command word.
Each supported form proposes exactly one separately authorized capability.

## Contract

- This file and the documents it links are the repository contract. Clone the shared workflow
  checkout for verification and name every gate you could not run.
- Core's [boundaries](https://github.com/dekopon-agents/dekopon/blob/main/AGENTS.md#boundaries-that-must-survive),
  [proportionate remedies](https://github.com/dekopon-agents/dekopon/blob/main/AGENTS.md#proportionate-remedies)
  and [Rust guidelines](https://github.com/dekopon-agents/dekopon/blob/main/AGENTS.md#rust-guidelines)
  apply here.
- Branch from `main` and open one ready PR at a time. Never push `main`. Tags, releases,
  version bumps or changes to repository settings require owner authorization.

## Boundaries particular to kubectl

- The command surface is closed to the readiness endpoint, the named Argo Application read and
  refresh annotation, and the named Deployment read. Each capability has its own id, manifest
  entry and grant.
- Validate name and namespace grammar. The API origin (`baseUrl`) and Argo namespace
  (`argoNamespace`) are owner settings. Require the Application `-n` to match `argoNamespace`.
  Paths, methods, annotation keys and output formats are fixed by capability.
- The component never sets `authorization` or handles token bytes. The broker injects a
  destination-bound service-account token at the native HTTP boundary.
- Project only the named status fields. Transport failure, non-2xx status and malformed
  response are typed errors. Secret, ConfigMap, exec, log and generic API forms are outside the
  command surface. Use offline component tests for all verification in this repository.

## Build and verify

A PR passes `ci / validate` before merge, the shared job in
[provider-workflows](https://github.com/dekopon-agents/provider-workflows). Run these steps:

```console
git clone --depth 1 https://github.com/dekopon-agents/provider-workflows ../provider-workflows
cargo fmt --all --check
cargo deny --all-features check bans licenses sources advisories
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked --package dekopon-kubectl-provider --lib --target wasm32-unknown-unknown -- -D warnings
cargo doc --locked --workspace --no-deps
../provider-workflows/build.sh
DEKOPON_PROVIDER_COMPONENT="$PWD/kubectl-provider.wasm" cargo test --locked --workspace
```

The toolchain and wasm-tools 1.259.0 are pinned for reproducible builds.

## Change rules

- Use comments only for the exceptions in core's
  [Comments](https://github.com/dekopon-agents/dekopon/blob/main/AGENTS.md#comments).
- Name tests for the invariant they exercise. Component tests read `DEKOPON_PROVIDER_COMPONENT`
  with `expect` and native-only tests say so in their names.
- Assert every request's full URI, method and body, exactly one call and the absence of guest
  `authorization`. Use the SDK testkit for component and native evidence.
- Leave `CHANGELOG.md` to release prep and record user-visible changes in physical `Changelog:`
  commit trailers.
- Move `dekopon-provider-sdk` and `dekopon-provider-sdk-testkit` together with exact version pins.
  Regenerate `Cargo.lock` with Cargo.
