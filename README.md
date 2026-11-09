# voice-to-tts

A simple Windows application to convert speech audio from a microphone (speech-to-text) into generated audio (text-to-speech).

## Virtual microphone (optional)

Without a virtual microphone, voice-to-tts plays the voice through your default audio output. To let a chat app hear it as if it were your microphone, install VB-CABLE, from [www.vb-cable.com](https://www.vb-cable.com).

1. Download the VB-CABLE driver pack from [www.vb-cable.com](https://www.vb-cable.com) and unzip it.
2. Right-click `VBCABLE_Setup_x64.exe`, choose Run as administrator, and click Install Driver.
3. Restart your computer.

Then pick `CABLE Output` as the microphone in your chat app (Discord, Zoom, Teams and so on). With VB-CABLE installed, voice-to-tts plays into `CABLE Input` and stays silent to you. Turn on "Hear the voice yourself" in the settings to also hear it through your default audio output.

You can also pick `CABLE Output` as the microphone of voice-to-tts itself, to speak what is sent into `CABLE Input`. It then plays the voice through your default audio output instead, so it does not hear itself.

## Speech models

Everything runs on your computer. The speech models (about 700 MB) are downloaded the first time the app starts and are stored in `%LOCALAPPDATA%\com.inforno.voice-to-tts\models`. [src-tauri/models.json](src-tauri/models.json) lists every file with its URL, size and SHA-256, and a download is only kept when it matches.

- Voice detection: [Silero VAD](https://github.com/snakers4/silero-vad) (MIT).
- Speech recognition: [Nemotron Speech Streaming](https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b) by NVIDIA. Licensed by NVIDIA Corporation under the NVIDIA Open Model License.
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

Note: This project was developed with the assistance of AI coding tools.
