# Summary

<!-- 1–3 bullets: what changed and why (not a file list) -->

-

## Tests

- [ ] `cargo test --locked --all-features --workspace`
- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets --all-features --workspace -- -D warnings`
- [ ] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --document-private-items --all-features --workspace --examples`
- [ ] Manual: `cargo run` — affected widgets still load / recover from errors as expected

## Notes

<!-- Beaking changes, config/env migrations, follow-ups, screenshots if UI-visible -->
