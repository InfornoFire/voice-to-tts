import { VB_CABLE_URL } from "../lib/links";
import ExternalLink from "./ExternalLink";

import "./About.css";

const CREDITS = [
  {
    name: "Nemotron Speech Streaming",
    href: "https://huggingface.co/nvidia/nemotron-speech-streaming-en-0.6b",
    role: "recognizes your speech",
    license: "NVIDIA Open Model License",
  },
  {
    name: "KittenTTS",
    href: "https://github.com/KittenML/KittenTTS",
    role: "speaks the voices",
    license: "Apache-2.0",
  },
  {
    name: "Silero VAD",
    href: "https://github.com/snakers4/silero-vad",
    role: "hears when you talk",
    license: "MIT",
  },
  {
    name: "sherpa-onnx",
    href: "https://github.com/k2-fsa/sherpa-onnx",
    role: "runs them all",
    license: "Apache-2.0",
  },
];

function About() {
  return (
    <section class="about" aria-labelledby="about-title">
      <h2 id="about-title" class="about-title">
        About
      </h2>
      <p class="hint">
        The optional virtual microphone is VB-CABLE, from{" "}
        <ExternalLink href={VB_CABLE_URL}>www.vb-cable.com</ExternalLink>.
      </p>
      <ul class="about-credits hint">
        {CREDITS.map((credit) => (
          <li key={credit.name}>
            <ExternalLink href={credit.href}>{credit.name}</ExternalLink> {credit.role} (
            {credit.license})
          </li>
        ))}
      </ul>
      <p class="hint">Licensed by NVIDIA Corporation under the NVIDIA Open Model License.</p>
    </section>
  );
}

export default About;
