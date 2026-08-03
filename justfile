_default:
    @just --list

# Prettier covers md/yaml/json. TOML is deliberately unchecked: taplo would be a
# new tool for four hand-maintained files. Add it when TOML volume justifies it.
prettier_glob := "**/*.{md,yml,yaml,json}"

# Format everything: prettier for md/yaml/json, cargo fmt for Rust
fmt: fmt-md fmt-rust

# Check everything without writing (non-zero exit on drift)
fmt-check: fmt-check-md fmt-check-rust

# Format md/yaml/json with prettier
fmt-md:
    npx --yes prettier@3 --write "{{ prettier_glob }}"

# Check md/yaml/json formatting
fmt-check-md:
    npx --yes prettier@3 --check "{{ prettier_glob }}"

# Format Rust with cargo fmt
fmt-rust:
    cargo fmt --all

# Rust formatting only (separate from prettier so a Rust gate never fails on Markdown drift)
fmt-check-rust:
    cargo fmt --all --check

# Format specific files: just fmt-file docs/a.md docs/b.md
fmt-file +files:
    npx --yes prettier@3 --ignore-unknown --write {{ files }}

# Check specific files: just fmt-check-file docs/a.md
fmt-check-file +files:
    npx --yes prettier@3 --ignore-unknown --check {{ files }}

# Build the workspace
build:
    cargo build --workspace --locked

# Run the CLI: just run -r some/dir (preview), just run -x -r some/dir (apply)
run *args:
    cargo run --locked -p detoxrs -- {{ args }}

# Run the workspace test suite
test:
    cargo test --workspace --locked

# Lint with clippy (all targets), treating warnings as errors
clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

# Verify the workspace builds on the declared MSRV (needs: rustup toolchain install 1.93.0)
msrv:
    cargo +1.93.0 build --workspace --locked

# Enforce the direct-dependency budget from proposal §7.2 (<= 11; transitive cap unmeasured)
dep-budget:
    #!/usr/bin/env python3
    # Counts [dependencies], [build-dependencies] and every
    # [target.*.dependencies]/[target.*.build-dependencies] table, in the
    # workspace root and each crate. Build- and target-gated deps still become
    # Debian source packages, which is what the budget exists to bound.
    # dev-dependencies are excluded: they never ship to a user.
    import tomllib, pathlib, sys

    limit, deps = 11, set()

    def harvest(tbl):
        deps.update(tbl.get("dependencies", {}))
        deps.update(tbl.get("build-dependencies", {}))
        for t in tbl.get("target", {}).values():
            deps.update(t.get("dependencies", {}))
            deps.update(t.get("build-dependencies", {}))

    # Only per-crate tables are counted. [workspace.dependencies] is a version
    # catalog, not a usage list: a crate opting in writes `foo = { workspace =
    # true }` in its OWN [dependencies], so `foo` is already a key there. Reading
    # the catalog directly would also count dev-dependencies declared in it
    # against the runtime budget, which is wrong.
    for m in [pathlib.Path("Cargo.toml"), *pathlib.Path("crates").glob("*/Cargo.toml")]:
        with m.open("rb") as f:
            harvest(tomllib.load(f))

    deps -= {p.name for p in pathlib.Path("crates").iterdir() if p.is_dir()}
    print(f"{len(deps)}/{limit} direct dependencies: {sorted(deps) or '(none)'}")
    sys.exit(1 if len(deps) > limit else 0)

# Advisory scan (cargo-audit)
audit:
    cargo audit

# License, ban, advisory and source policy (cargo-deny)
deny:
    cargo deny check

# Dependency review status (cargo-vet)
vet:
    cargo vet check

# Report unsafe usage in the dependency tree (cargo-geiger; needs an absolute
# --manifest-path: it refuses both a virtual manifest and a relative path)
geiger:
    cargo geiger --manifest-path "{{ justfile_directory() }}/crates/detoxrs/Cargo.toml"

# Filesystem vulnerability/secret scan (trivy)
trivy:
    trivy fs --scanners vuln,secret,misconfig .

# Generate a CycloneDX SBOM (cargo-cyclonedx)
sbom:
    cargo cyclonedx --format json --all

# Fast local gate: what a developer runs before pushing
gate: fmt-check clippy test msrv dep-budget

# Full gate incl. supply chain (needs cargo-audit/deny/vet/geiger + trivy installed)
ci: gate audit deny vet geiger trivy
