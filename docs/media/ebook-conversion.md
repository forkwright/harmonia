# Ebook conversion

Harmonia's ebook conversion crate is a controlled subprocess boundary. It does
not vendor conversion engines into the Rust workspace; it shells out to the
following binaries:

| Binary | Nix package | Used for |
| --- | --- | --- |
| `ebook-convert` | `calibre` | General ebook conversion, including non-Kobo EPUB output |
| `kepubify` | `kepubify` | EPUB to Kobo KEPUB conversion |
| `pandoc` | `pandoc` | DOCX/ODT to EPUB conversion |

The canonical deployment path is the Harmonia flake package. The native Nix
package wraps `$out/bin/harmonia` with a `PATH` that includes those three
packages, so production deployments do not depend on host-global binaries.
Versions are pinned by the repository `flake.lock` through the `nixpkgs` input.

Development shells include the same packages. Non-Nix deployments must provide
compatible binaries on `PATH`; otherwise conversion fails with
`ConvertError::BinaryNotFound` before any output file is reported as successful.

These tools are runtime dependencies for conversion only. They are deliberately
not Cargo dependencies, and `cargo-deny` cannot audit or pin them.
