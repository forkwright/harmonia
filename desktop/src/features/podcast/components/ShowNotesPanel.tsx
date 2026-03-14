/** Sanitized HTML show notes renderer. Links open in system browser. */

import { useEffect, useRef, useMemo } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";

interface Props {
  html: string | null;
}

const DANGEROUS_TAGS = ["script", "iframe", "form", "object", "embed", "meta", "link", "base"];

function sanitizeHtml(dirty: string): string {
  const doc = new DOMParser().parseFromString(dirty, "text/html");

  DANGEROUS_TAGS.forEach((tag) => {
    doc.querySelectorAll(tag).forEach((el) => el.remove());
  });

  doc.querySelectorAll("*").forEach((el) => {
    Array.from(el.attributes).forEach((attr) => {
      const name = attr.name.toLowerCase();
      const value = attr.value.toLowerCase().trimStart();
      if (
        name.startsWith("on") ||
        (name === "href" && value.startsWith("javascript:")) ||
        (name === "src" && value.startsWith("javascript:")) ||
        name === "srcdoc"
      ) {
        el.removeAttribute(attr.name);
      }
    });
  });

  return doc.body.innerHTML;
}

export default function ShowNotesPanel({ html }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  const sanitized = useMemo(() => (html ? sanitizeHtml(html) : null), [html]);

  // Intercept all link clicks and open in system browser.
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const handler = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      const anchor = target.closest("a");
      if (!anchor) return;
      const href = anchor.getAttribute("href");
      if (!href) return;
      e.preventDefault();
      void openUrl(href);
    };

    container.addEventListener("click", handler);
    return () => container.removeEventListener("click", handler);
  }, [sanitized]);

  if (!sanitized) {
    return <p className="text-sm text-gray-500 italic">No show notes available.</p>;
  }

  return (
    <div
      ref={containerRef}
      className="prose prose-invert prose-sm max-w-none text-gray-300"
      // WHY: dangerouslySetInnerHTML is safe here because sanitizeHtml strips all
      // script tags, event handlers, and javascript: URLs before rendering.
      dangerouslySetInnerHTML={{ __html: sanitized }}
    />
  );
}
