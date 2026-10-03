# Changelog

## v0.1.0

Initial release.

- Installs and runs upm (1.0.1 and newer) on proto's Node, with `upm` and `upx` shims.
- Lists versions and every npm dist-tag as an alias; `latest` follows npm's `latest`.
- Verifies the tarball against the registry's sha512 integrity and records it in the lockfile.
- `registry-url`, `dist-url` and `upx-shim` settings under `[tools.upm]`.
- Detects the version from `package.json` and pins to `devEngines.packageManager` with `proto pin --tool-native`.
- Warns once after install when proto's Node is below 22.3.
