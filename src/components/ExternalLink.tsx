import type { ComponentChildren } from "preact";

import { openLink } from "../lib/links";

interface ExternalLinkProps {
  href: string;
  children: ComponentChildren;
}

function ExternalLink({ href, children }: ExternalLinkProps) {
  const open = (event: MouseEvent) => {
    event.preventDefault();
    openLink(href);
  };

  return (
    <a href={href} onClick={open}>
      {children}
    </a>
  );
}

export default ExternalLink;
