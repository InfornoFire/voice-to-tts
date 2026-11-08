import "./SignalPath.css";

const STOPS = [
  { name: "Your microphone", detail: "You speak into it as usual" },
  { name: "voice-to-tts", detail: "Turns your words into a computer voice" },
  { name: "CABLE Output", detail: "Choose this as the microphone in your chat app" },
  { name: "Your chat app", detail: "Discord, Zoom, Teams and so on" },
];

/** Shows where the voice goes, from the real microphone to the chat app. */
function SignalPath() {
  return (
    <ol class="signal-path" aria-label="Where your voice goes">
      {STOPS.map((stop) => (
        <li key={stop.name} class="signal-path-stop">
          <span class="signal-path-name">{stop.name}</span>
          <span class="hint">{stop.detail}</span>
        </li>
      ))}
    </ol>
  );
}

export default SignalPath;
