import React, {useEffect, useRef} from 'react';
import {useColorMode} from '@docusaurus/theme-common';

/**
 * Renders an SVG that uses `currentColor` in a color-mode-aware way.
 *
 * Loads the SVG via fetch, injects it into the DOM as inline SVG,
 * and wraps it in a container whose CSS `color` matches the active theme.
 *
 * Usage in MDX:
 *   import ThemedSvg from '@site/src/components/ThemedSvg';
 *   <ThemedSvg src="/img/diagrams/mental-model/core-loop.svg" alt="Core Loop" />
 */
export default function ThemedSvg({
  src,
  alt,
  maxWidth = '600px',
}: {
  src: string;
  alt: string;
  maxWidth?: string;
}) {
  const {colorMode} = useColorMode();
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    fetch(src)
      .then((res) => res.text())
      .then((svgText) => {
        if (containerRef.current) {
          containerRef.current.innerHTML = svgText;
          const svg = containerRef.current.querySelector('svg');
          if (svg) {
            svg.style.width = '100%';
            svg.style.height = 'auto';
            svg.setAttribute('role', 'img');
            svg.setAttribute('aria-label', alt);
          }
        }
      });
  }, [src, alt]);

  return (
    <div
      ref={containerRef}
      style={{
        color: colorMode === 'dark' ? '#e3e3e3' : '#1b1b1b',
        maxWidth,
        margin: '1.5rem auto',
      }}
    />
  );
}
