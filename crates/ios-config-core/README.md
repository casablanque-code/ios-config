# ios-config-core

Intermediate representation (IR) types for parsed Cisco IOS configurations.

This crate is a companion to [`ios-config`](https://crates.io/crates/ios-config)
and is re-exported from it. You typically do not need to depend on this crate
directly — depend on `ios-config` instead and use the types from there.

```toml
[dependencies]
ios-config = "0.1"
```

If you are building a tool that only needs the IR types (e.g. a renderer or
a converter) without pulling in the parser, you can depend on this crate alone:

```toml
[dependencies]
ios-config-core = "0.1"
```
