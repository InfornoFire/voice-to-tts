import type { ComponentChildren } from "preact";

import "./Setting.css";

interface SettingProps {
  label: string;
  /** Shown beside the label, such as the current value of a slider. */
  value?: string;
  hint?: string;
  children: ComponentChildren;
}

function Setting({ label, value, hint, children }: SettingProps) {
  return (
    <div class="setting">
      <label class="setting-label">
        <span class="setting-heading">
          <span>{label}</span>
          {value && <span class="setting-value">{value}</span>}
        </span>
        {children}
      </label>
      {hint && <p class="hint">{hint}</p>}
    </div>
  );
}

export default Setting;
