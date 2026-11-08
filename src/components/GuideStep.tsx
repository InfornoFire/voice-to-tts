import type { ComponentChildren } from "preact";

import "./GuideStep.css";

interface GuideStepProps {
  number: number;
  title: string;
  children: ComponentChildren;
}

function GuideStep({ number, title, children }: GuideStepProps) {
  return (
    <section class="guide-step">
      <h2 class="guide-step-title">
        <span class="guide-step-number" aria-hidden="true">
          {number}
        </span>
        {title}
      </h2>
      {children}
    </section>
  );
}

export default GuideStep;
