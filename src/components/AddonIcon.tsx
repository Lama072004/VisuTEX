/** Symbol eines Add-ons bzw. Menüband-Eintrags: kurzer Text (z. B. „Ω“) oder SVG-Datei. */
import type { ReactNode } from "react";

export const isSvgIcon = (icon: string) => icon.toLowerCase().endsWith(".svg");

const monochromeCache = new Map<string, boolean>();

/** Einfarbige Symbole (`currentColor`) folgen der Textfarbe – auch im dunklen Design. */
function isMonochrome(url: string) {
  let result = monochromeCache.get(url);
  if (result === undefined) {
    try {
      result = atob(url.slice(url.indexOf(",") + 1)).includes("currentColor");
    } catch {
      result = false;
    }
    monochromeCache.set(url, result);
  }
  return result;
}

type Props = {
  icon: string;
  /** Pfad im Add-on → data:-URL (von Rust geprüft) */
  icons: Record<string, string>;
  fallback?: ReactNode;
  className?: string;
};

export function AddonIcon({ icon, icons, fallback = "⧉", className = "addon-icon" }: Props) {
  if (isSvgIcon(icon)) {
    const url = icons[icon];
    if (!url) return <span className={className}>{fallback}</span>;
    if (isMonochrome(url)) {
      // Als Maske dargestellt: Form aus der SVG, Farbe aus `currentColor`
      const mask = `url("${url}")`;
      return <span className={`${className} svg mask`} style={{ maskImage: mask, WebkitMaskImage: mask }} aria-hidden />;
    }
    return <img className={`${className} svg`} src={url} alt="" draggable={false} />;
  }
  return <span className={className}>{icon || fallback}</span>;
}
