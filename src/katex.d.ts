declare module "katex" {
  export function renderToString(
    expression: string,
    options?: {
      displayMode?: boolean;
      throwOnError?: boolean;
      strict?: "warn" | "ignore" | "error";
      trust?: boolean;
      /** Eigene Makros, z. B. { "\\R": "\\mathbb{R}" } */
      macros?: Record<string, string>;
    },
  ): string;

  const katex: { renderToString: typeof renderToString };
  export default katex;
}
