# voice-to-tts

A simple Windows application to convert speech audio from a microphone (speech-to-text) into generated audio (text-to-speech).

## Virtual microphone

The app speaks into a virtual microphone. It is VB-CABLE, from [www.vb-cable.com](https://www.vb-cable.com).

1. Download the VB-CABLE driver pack from [www.vb-cable.com](https://www.vb-cable.com) and unzip it.
2. Right-click `VBCABLE_Setup_x64.exe`, choose Run as administrator, and click Install Driver.
3. Restart your computer.

Then pick `CABLE Output` as the microphone in your chat app (Discord, Zoom, Teams and so on). voice-to-tts only plays into `CABLE Input` and never into your speakers.

## Speech models

Everything runs on your computer. The speech models (about 100 MB) are downloaded the first time the app starts and are stored in `%LOCALAPPDATA%\com.inforno.voice-to-tts\models`. [src-tauri/models.json](src-tauri/models.json) lists every file with its URL, size and SHA-256, and a download is only kept when it matches.

- Voice detection: [Silero VAD](https://github.com/snakers4/silero-vad) (MIT).
- Speech recognition: [Kroko ASR](https://huggingface.co/Banafo/Kroko-ASR) by Banafo (CC-BY-SA).
- Voices: [KittenTTS](https://github.com/KittenML/KittenTTS) nano (Apache-2.0).
- All of them run with [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx).

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
