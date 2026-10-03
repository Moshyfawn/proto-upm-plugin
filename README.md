# proto-upm-plugin

Install and pin [upm](https://github.com/unjs/upm) with [proto](https://moonrepo.dev/proto). upm runs on proto's Node, and its version can be pinned or detected from your project.

- **Pinnable.** Pin upm in `.prototools` or `package.json` so your team and CI run the same version.
- **Verified.** Every download is checked against the npm registry's sha512 integrity and recorded in proto's lockfile.
- **Two shims.** `upm` and `upx` land on PATH. `upx` can be switched off if it clashes with the UPX packer.

```sh
proto plugin add upm github://Moshyfawn/proto-upm-plugin
proto install upm
```

upm needs Node 22.3 or newer. On older versions it exits silently, so the plugin warns once after install when proto's Node is too old.

## Pinning

In `.prototools`:

```toml
upm = "<version>"

[plugins.tools]
upm = "github://Moshyfawn/proto-upm-plugin"
```

Or in `package.json`, read in this order: `devEngines.packageManager`, `packageManager`, `volta.upm`, `engines.upm`.

```json
{ "devEngines": { "packageManager": { "name": "upm", "version": "<version>" } } }
```

```sh
proto pin upm <version> --tool-native   # writes devEngines.packageManager
proto unpin upm --tool-native           # removes it
proto versions upm --aliases            # lists what you can install
```

## Settings

Under `[tools.upm]` in `.prototools`:

| Key | Default | |
| --- | --- | --- |
| `registry-url` | `https://registry.npmjs.org` | Where versions and integrity hashes come from |
| `dist-url` | `{registry}/{package}/-/{package_without_scope}-{version}.tgz` | Tarball URL. `{file}` is also available |
| `upx-shim` | `true` | Set to `false` to keep `upx` off PATH, then run `proto regen` |

A misspelled key fails with an error that lists the valid ones. upm rejects `-g`, so there is no global package support.

## Development

```sh
cargo wasm     # build the plugin
cargo t        # integration tests, run against the built .wasm
tests/e2e.sh   # install and run upm through a real proto
```
