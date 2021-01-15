# prc-rs

Rust crate for handling `.prc` filetypes in Smash Ultimate. Provides access to reading/writing methods in IO or the filesystem, as well as a simple API for interacting with or manipulating param data.

To add this as a dependency:

```toml
[dependencies]
prc-rs = "1.3"
```

The `xml` feature flag is available to expose methods allowing params to be converted into and out of XML format.

### Crates.io

[crates.io](https://crates.io/crates/prc-rs)

### Documentation

[rust-doc](https://docs.rs/prc-rs/)

# param-xml

A runtime utilizing the XML dependency to convert param files to XML and back. See the `Releases` section for Windows builds.

Install with cargo via `cargo install param-xml`
