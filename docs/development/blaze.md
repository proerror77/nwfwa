# Optional Blaze development loop

Blaze is an experimental local option for the `fwa-rules`, `fwa-core`, and
`fwa-features` library unit tests. It is not required by this repository.
The existing Cargo checks, Rust toolchain, CI, release, and deployment commands
remain unchanged. The helper does not start the API, database, or ML service.

Install Blaze separately using its [official installation instructions](https://github.com/indexable-inc/blaze#install).
The helper never installs software or changes your shell's Cargo command. Read
the current [compatibility notes](https://github.com/indexable-inc/blaze/blob/main/docs/cargo.md)
before choosing a version: Blaze uses its own toolchain and does not validate the
repository's `rust-toolchain.toml` toolchain. Its checks are not equivalent to rustc.

From the repository root:

```sh
scripts/dev/blaze_test.sh                     # fwa-rules library unit tests
scripts/dev/blaze_test.sh fwa-core
scripts/dev/blaze_test.sh fwa-features name_fragment
```

A missing Blaze executable is an explicit error, not a silent Cargo fallback.
The helper pins dependency resolution with `--locked`, selects one library, and
sets `BLAZE_TESTS=all` so that a comparison does not silently skip previously
passing tests. It accepts an optional test-name filter, not arbitrary Cargo flags.

Before committing, use the original toolchain for the affected package:

```sh
cargo check --locked -p fwa-rules
cargo test --locked -p fwa-rules --lib
```

Replace the package and use the same test filter for a direct comparison. Follow
the normal repository validation cadence and CI for broader checks. An unsupported
Blaze build should be investigated or rerun explicitly with Cargo; do not alter
production dependencies, toolchain pins, or release profiles just to make it pass.

## Verified compatibility (2026-10-29)

On macOS arm64 at revision `8c44516a`, both Cargo and Blaze canary
`0.20261029.0` passed the same nine `fwa-rules` library tests. Stable Blaze
`0.20261027.0` failed in the standard library test harness with the writable static
`__RUST_STD_INTERNAL_VAL`; it is not a working version for this trial.

Keep canary separate from the normal Cargo and stable Blaze commands. If it is
installed in `$HOME/.local/share/blaze-canary/bin`, select it for one invocation:

```sh
PATH="$HOME/.local/share/blaze-canary/bin:$PATH" scripts/dev/blaze_test.sh
```

The installed canary is a prerelease. Record the version when reproducing results;
future stable or canary releases need their own verification. The observed first
runs took 65.038 seconds with Cargo and 51.321 seconds with canary, but the canary
run reused dependency caches from another project. These are not clean-build or
matched-cache measurements and do not establish a speedup. Edited-function rebuild
performance remains to be measured separately.

## Evaluate before expanding

Record the Blaze version, Cargo/rustc versions, source revision, package, test
filter, first-build time, edited-function rebuild time, and memory use. Compare
identical inputs and confirm that the same tests pass. Keep build commands serial
and start with one package on memory-constrained machines. The verified result above covers only the named package and version; it does not
establish compatibility for the whole workspace or any particular speedup.

The ONNX/ML crates, API workers, Yew/WASM frontend, release artifacts, and runtime
hot updates are outside this trial. Stop Blaze using `blaze stop` when finished;
return to the existing Cargo commands at any time.
