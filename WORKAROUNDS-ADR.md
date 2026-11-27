# Workarounds

- `frontend/package.json` overrides KaTeX to `^0.19.1`. `micromark-extension-math` 3.1.0 still requests `^0.16.0`, which
  resolves to a version affected by [GHSA-238p-pmpm-9mq7](https://github.com/advisories/GHSA-238p-pmpm-9mq7). Remove the
  override when the upstream range resolves to a patched version and `npm audit` passes without it.
- `frontend/package.json` overrides `re2js` to `^2.10.0` because `markdownlint` 0.42.2 pins 2.9.1. Remove the override
  when `markdownlint` resolves to 2.10.0 or newer on its own and Markdown linting still passes.
- `docker/production/production.Dockerfile` builds empty Rust targets and removes their artifacts to cache only the
  release dependencies. Cargo 1.100 has no stable
  [dependencies-only build](https://github.com/rust-lang/cargo/issues/2644). Remove the placeholder targets when Cargo
  can populate the layer directly. Verify that a fresh BuildKit builder can import the layer, rebuild just the app after
  a source change, and run the new binary.
