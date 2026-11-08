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

To try the app without the speech models or a microphone, run it against a scripted engine:

```sh
pnpm tauri dev --features mock-engine
```

Logs are written to `%LOCALAPPDATA%\com.inforno.voice-to-tts\logs` (no speech logs).

Checks:

```sh
pnpm run lint
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo test --manifest-path src-tauri/Cargo.toml
```
