import { VB_CABLE_URL } from "../lib/links";
import ExternalLink from "./ExternalLink";

import "./About.css";

const CREDITS = [
  {
    name: "Kroko ASR",
    href: "https://huggingface.co/Banafo/Kroko-ASR",
    role: "recognizes your speech",
    license: "CC-BY-SA",
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
        The virtual microphone is VB-CABLE, from{" "}
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
    </section>
  );
}

export default About;
