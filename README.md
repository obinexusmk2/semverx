# semverx

> **Semantic Versioning eXtended** — polyglot package metadata and dependency
> resolution for the OBINexus toolchain.
>
> *When an ecosystem decoheres, SemverX actively rectifies the codebase.*

[![crate](https://img.shields.io/badge/crate-semverx-orange)](https://github.com/obinexus/semverx)
[![registry](https://img.shields.io/badge/registry-r.obinexus.org-blue)](https://r.obinexus.org)
[![license](https://img.shields.io/badge/license-MIT-green)](./LICENSE)
[![YouTube](https://img.shields.io/badge/YouTube-%40OBINexus-red)](https://youtube.com/@OBINexus)

---

## Status

This repository is in **recovery build — v0.1.0**. The surface shipping today:

- `core::Version` — SemVer 2.0 parser (major.minor.patch + pre + build).
- `resolver::DependencyResolver` trait + `GraphResolver` / `SemverXResolver`
  backed by `petgraph` with a dijkstra shortest-path implementation.
- `ffi::` — `#[repr(C)]` polyglot ABI: `semverx_parse`, `semverx_free`,
  `semverx_compare`, `semverx_default_mode`, `semverx_binding_mode_name`,
  `semverx_string_free`, plus the `BindingMode { Monoglot, Polyglot, Hybrid }`
  discriminator.

See [Roadmap](#roadmap) for the hot-swap / HDIS / verb-noun-class layers
that are in design but not yet wired into `lib`.

---

## What SemverX is

Traditional SemVer tells you **what** changed between two releases. SemverX
extends the schema so a consumer can decide **how to update safely** across a
polyglot toolchain, without dismantling the whole system.

The extended schema layered on top of `major.minor.patch`:

| Axis         | Values                                                     |
|--------------|------------------------------------------------------------|
| classifier   | `stable`, `legacy`, `experimental`, `deprecated`           |
| environment  | `dev`, `test`, `staging`, `prod`                           |
| binding mode | `monoglot`, `polyglot`, `hybrid`                           |

The classifier drives the migration ladder (stable → beta → alpha) under the
OBINexus QA methodology: **TDD, not BDD**. Every quality bar is asserted
before a component is allowed to cross into production.

---

## Architecture

### Tri-binding model

Mirrors the registry schema `@obinexus/<pkg>.{monoglot,polyglot,hybrid}`:

- **Monoglot** — a single-language binding. The consumer is locked to one
  language; no foreign-call bridging is advertised.
- **Polyglot** — the standardised C-ABI surface, inspired by `libpolycall`.
  The same `libsemverx.{so,dylib,dll}` / `libsemverx.a` artefact is consumed
  from Python (`ctypes`), Node.js (`ffi-napi` / N-API), PHP (`FFI::cdef`),
  Lua (`ffi.cdef`), and any language that can call C.
- **Hybrid** — monoglot host with polyglot extensions. The default, because
  most real systems run one primary language and delegate specialised work
  across the FFI boundary.

### Tri-topology resolver

Dependency resolution picks the traversal strategy based on graph shape:

- **Eulerian cycle** — visit all edges (comprehensive audit).
- **Hamiltonian cycle** — visit all nodes (direct, node-weight-driven).
- **A\* scoring** — nearest viable path with an admissible heuristic
  (optimal for hot paths).
- **Hybrid** — adaptive; switches strategy as graph stress changes.

The target is `O(log n)` resolution over the working set, with an auxiliary
index keeping the steady-state cost at `O(1)` for already-resolved
components — so a human is not in the loop for a well-known upgrade.

### Tri-registry storage

Borrowed from the fault-tolerant P2P topology in the architecture talks,
the registry ships in three layers:

- **Local** — `git-raw` (regulated firmware) on the developer's machine;
  live-updatable.
- **Remote** — timestamped, metadata-alpha mirror. The authoritative live
  copy.
- **Archive** — static, stable major-patch snapshot. Auto-resolved; the
  consumer is locked in once pinned.

This matches the consent / permission / consensus voting model: any publish
path must satisfy all three (local write, remote mirror write, archive
commit) before a version is considered globally visible.

---

## Quick start

### Build

```bash
git clone https://github.com/obinexus/semverx
cd semverx
cargo build --release
```

Produces:

- `target/release/semverx` — the CLI binary.
- `target/release/libsemverx.{a,so,dylib,dll}` — cdylib + staticlib for
  polyglot consumers (see `crate-type = ["lib", "cdylib", "staticlib"]` in
  `Cargo.toml`).

### Rust API

```rust
use semverx::core::Version;
use semverx::resolver::{Component, SemverXResolver};

let v = Version::parse("1.2.3-beta.1+build.42")?;
assert_eq!((v.major, v.minor, v.patch), (1, 2, 3));

let mut resolver = SemverXResolver::new();
resolver.add_package(Component::new("foo", "1.0.0"));
resolver.add_package(Component::new("bar", "2.0.0"));
resolver.add_dependency("foo", "bar");

let resolved = resolver.resolve("foo", "1.0.0")?;
```

### FFI (polyglot consumers)

The same `libsemverx` artefact is callable from any C-ABI-capable language.

```c
/* C */
#include <stdio.h>
#include "semverx.h"

int main(void) {
    CVersion *v = semverx_parse("1.2.3");
    if (v) {
        printf("%llu.%llu.%llu\n", v->major, v->minor, v->patch);
        semverx_free(v);
    }
    return 0;
}
```

```python
# Python (ctypes)
import ctypes
lib = ctypes.CDLL("./target/release/libsemverx.so")
lib.semverx_parse.restype = ctypes.c_void_p
lib.semverx_free.argtypes = [ctypes.c_void_p]

v = lib.semverx_parse(b"1.2.3")
lib.semverx_free(v)
```

```javascript
// Node.js (ffi-napi)
const ffi = require('ffi-napi');
const lib = ffi.Library('./target/release/libsemverx', {
    'semverx_parse':   ['pointer', ['string']],
    'semverx_compare': ['int',     ['pointer', 'pointer']],
    'semverx_free':    ['void',    ['pointer']],
});
```

---

## Library surface

```
semverx
├── core
│   ├── Version            (parser, Ord, Display)
│   └── OBINexusSemverX    (security-mode wrapper)
├── resolver
│   ├── DependencyResolver (trait)
│   ├── Component          (name, version, dependencies)
│   ├── ResolutionError
│   └── graph
│       ├── GraphResolver
│       └── SemverXResolver
├── normalizer
│   └── normalize_unicode_path
└── ffi                    (#[repr(C)] polyglot ABI)
    ├── BindingMode        { Monoglot, Polyglot, Hybrid }
    ├── CVersion
    ├── semverx_parse / semverx_free
    ├── semverx_compare
    ├── semverx_default_mode
    ├── semverx_binding_mode_name
    └── semverx_string_free
```

---

## Windows / WSL

All shell scripts in this repo use LF line endings. If you're editing on
Windows, make sure your editor does not re-save them as CRLF, or WSL bash
will print `$'\r': command not found` and abort on the first `if` block.

A `.gitattributes` entry at the repo root is recommended:

```
*.sh        text eol=lf
*.rs        text eol=lf
Cargo.toml  text eol=lf
```

For full MSYS2 / MinGW setup (linker flags, `cbindgen`, CMake shims for the
C components under `docs/MVP-SemverX/`), see
[MSYS_RUST_SETUP.md](./MSYS_RUST_SETUP.md).

---

## OBINexus toolchain integration

semverx sits inside the wider OBINexus stack:

- **Language layer:** `riftlang.exe` → `.so.a` → `rift.exe` → `gosilang`.
  SemverX is how a component built in any of those stages is registered,
  pinned, and hot-swapped downstream.
- **Build orchestration:** `nlink` → `polybuild`. The SemverX CLI integrates
  with polybuild pre- and post-processing hooks for dependency audit,
  compatibility matrix generation, and hot-swap preparation.
- **Registry:** `r.obinexus.org`, indexed by
  `@obinexus/<pkg>.{monoglot,polyglot,hybrid}`.

---

## Roadmap

The following surfaces are designed (see `src/resolver/{errors,types,
strategies,stress}.rs` and `src/ffi/c_api.rs`) but are **not yet wired into
the `lib` crate**. They depend on a superset type system (`SemverX` struct,
`BubblingError`, `ComponentHealth`, `StressZone`, `SEIMetadata`,
`Environment`, `Classifier`, `VerbNounClass`) queued for the next milestone:

- SEI metadata (Statement / Expression / Intent) contracts per component.
- Stress-zone-aware self-healing (`Ok` / `Warning` / `Danger` / `Critical`).
- Hot-swap executor with automatic rollback.
- HDIS (Hybrid Directed Instruction System) coherence targeting.
- Full `c_api.rs` polyglot FFI for the extended type system.
- CLI subcommands: `resolve`, `health`, `audit`, `generate`, `polybuild`.

These are tracked against the OBINexus milestone-based investment policy
and the `#NoGhosting` / `OpenSense` recruitment commitment — milestones are
shipped or the repo surfaces the gap, not hidden.

---

## Contributing

- Open a PR against `main`. TDD is required: new behaviour comes with a
  failing test first.
- Shell scripts must stay LF-terminated (`.gitattributes` enforces this on
  checkout, but check your local editor).
- No Unicode arrows or glyphs in code files; plain ASCII keeps the build
  portable across MSYS2, WSL, and CI runners.

### Local dev loop

```bash
cargo check          # fast syntax + type check
cargo build --lib    # library only (fastest useful build)
cargo test
cargo clippy
cargo fmt --check
```

---

## License

MIT — OBINexus Computing.

---

## Links

- **GitHub:** [obinexus/semverx](https://github.com/obinexus/semverx)
- **Registry:** [r.obinexus.org](https://r.obinexus.org)
- **YouTube:** [@OBINexus](https://youtube.com/@OBINexus)
- **Related:** [`libpolycall`](https://github.com/obinexus/libpolycall),
  [`hdis`](https://github.com/obinexus/hdis)

---

**Author:** Nnamdi Michael Okpala — [@obinexus](https://github.com/obinexus) · okpalan@protonmail.com
