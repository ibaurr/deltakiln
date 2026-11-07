# deltakiln

An incremental-first static site generator for very large markdown doc sets,
written in Rust.

The goal is not raw per-file render speed. It is a persisted dependency graph
plus content-addressed caching, so that editing one file rebuilds only what
actually changed and everything truly downstream of it, and the result is
verifiably never stale.

## Status: early development (M0)

Only the foundation exists today. **Incremental builds, the cache, `serve`,
`verify`, `explain` and `bench` are not implemented yet.** Every build is a
full from-scratch build.

That full build is deliberate: it is the correctness oracle that later
milestones will compare incremental output against. No performance claims are
made yet. Any performance targets are proposals to be validated by a benchmark
harness (milestone M4), not measured results.

| Milestone | Scope | State |
| --- | --- | --- |
| M0 | Workspace, CLI, `init`, naive full build | done |
| M1 | Hashing, graph, persisted cache, `verify` | planned |
| M2 | Derived nodes, output-based invalidation | planned |
| M3 | `serve`, file watching, live reload, `explain` | planned |
| M4 | Benchmark harness and first published numbers | planned |
| M5 | Sitemap, RSS, search shards, highlighting, docs site, v0.1 | planned |

## Try it

Requires a recent stable Rust toolchain.

```sh
cargo build --release
./target/release/deltakiln init mysite
./target/release/deltakiln build --root mysite
```

`init` creates a starter site:

```
deltakiln.toml            # optional; every field has a default
templates/page.html       # the layout (a built-in one is used if absent)
content/index.md          # -> /
content/guide/getting-started.md   # -> /guide/getting-started/
static/                   # copied to the output as-is (optional)
```

Output goes to `public/` by default (`--output` to change it).

### Content model

- Markdown with optional TOML front matter between `+++` lines. Supported
  keys: `title`, `description`, `date`, `slug`, `url`, `template`, and a free
  `[extra]` table. Unknown keys are an error. YAML front matter is not
  supported yet.
- Clean URLs: `a/b.md` is written to `a/b/index.html`, and `index.md` maps to
  its directory. `slug` replaces the last path segment; `url` replaces the
  whole path.
- Templates use [minijinja](https://github.com/mitsuhiko/minijinja) syntax
  and can read only the page's own data (`page.*`) and the site config
  (`site.*`).
- Links are passed through as written. An `@/path/to/page.md` link syntax
  with build-time broken-link checking is planned for a later milestone.

## Development

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The golden test compares the fixture site against
`crates/deltakiln-render/tests/fixtures/basic.expected/`. After an intentional
output change, regenerate it with
`UPDATE_GOLDEN=1 cargo test -p deltakiln-render --test golden` and review the
diff.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this project by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
