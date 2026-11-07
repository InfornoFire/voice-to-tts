# voice-to-tts

A simple Windows application to convert speech audio from a microphone (speech-to-text) into generated audio (text-to-speech).

## Virtual microphone

The virtual microphone is provided by VB-CABLE from [www.vb-cable.com](https://www.vb-cable.com). VB-CABLE is donationware and all contributions are welcome.

## Development

Requirements: Windows 10 or 11 x64, Rust, the Visual Studio 2026 Build Tools (MSVC with the Windows SDK), and pnpm.

```sh
pnpm install
pnpm tauri dev
```

Checks:

```sh
pnpm run lint
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```
