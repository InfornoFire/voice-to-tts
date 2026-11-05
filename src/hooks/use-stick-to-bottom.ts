import { useLayoutEffect, useRef, useState } from "preact/hooks";

const BOTTOM_TOLERANCE_PX = 24;

/** Keeps a scroll container at its bottom as `content` changes, until the user scrolls up. */
export function useStickToBottom(content: unknown) {
  const ref = useRef<HTMLElement>(null);
  const [stuck, setStuck] = useState(true);

  useLayoutEffect(() => {
    const element = ref.current;
    if (element && stuck) {
      element.scrollTop = element.scrollHeight;
    }
  }, [content, stuck]);

  const onScroll = () => {
    const element = ref.current;
    if (element) {
      const distanceFromBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
      setStuck(distanceFromBottom <= BOTTOM_TOLERANCE_PX);
    }
  };

  return { ref, onScroll };
}
