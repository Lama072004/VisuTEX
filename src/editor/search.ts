/**
 * Suchen & Ersetzen im Dokument – arbeitet auf den Textblöcken des Editors
 * (nicht auf HTML), hebt Treffer per Dekoration hervor und ersetzt per
 * Transaktion (rückwärts, damit Positionen gültig bleiben). Formatierungen
 * bleiben erhalten.
 */
import { Extension } from "@tiptap/core";
import type { Node as PMNode } from "@tiptap/pm/model";
import { Plugin, PluginKey, TextSelection } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";

export type SearchOptions = { caseSensitive: boolean; wholeWord: boolean };
export type Match = { from: number; to: number };

type SearchState = {
  query: string;
  options: SearchOptions;
  matches: Match[];
  current: number;
  decorations: DecorationSet;
};

export const searchKey = new PluginKey<SearchState>("vtxSearch");

const ATOM_PLACEHOLDER = "￼";

function escapeRegExp(text: string) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

export function findMatches(doc: PMNode, query: string, options: SearchOptions): Match[] {
  if (!query) return [];
  const flags = options.caseSensitive ? "gu" : "giu";
  // Kein Lookbehind im Muster: ältere WebKit-Versionen (macOS/Linux) unterstützen ihn nicht.
  const pattern = new RegExp(escapeRegExp(query), flags);
  const wordChar = /[\p{L}\p{N}_]/u;
  const matches: Match[] = [];
  doc.descendants((node, pos) => {
    if (!node.isTextblock) return true;
    let text = "";
    const offsets: number[] = [];
    node.forEach((child, childOffset) => {
      const start = pos + 1 + childOffset;
      if (child.isText) {
        const value = child.text ?? "";
        for (let index = 0; index < value.length; index += 1) offsets.push(start + index);
        text += value;
      } else {
        offsets.push(start);
        text += ATOM_PLACEHOLDER;
      }
    });
    offsets.push(pos + node.nodeSize - 1);
    for (const match of text.matchAll(pattern)) {
      const index = match.index ?? 0;
      if (match[0].length === 0) continue;
      if (options.wholeWord) {
        const before = text[index - 1] ?? "";
        const after = text[index + match[0].length] ?? "";
        if ((before && wordChar.test(before)) || (after && wordChar.test(after))) continue;
      }
      matches.push({ from: offsets[index], to: offsets[index + match[0].length - 1] + 1 });
    }
    return false;
  });
  return matches;
}

function decorate(doc: PMNode, matches: Match[], current: number) {
  return DecorationSet.create(
    doc,
    matches.map((match, index) =>
      Decoration.inline(match.from, match.to, { class: index === current ? "search-match search-match-current" : "search-match" }),
    ),
  );
}

declare module "@tiptap/core" {
  interface Commands<ReturnType> {
    searchReplace: {
      setSearch: (query: string, options: SearchOptions) => ReturnType;
      clearSearch: () => ReturnType;
      gotoMatch: (direction: 1 | -1) => ReturnType;
      replaceCurrent: (replacement: string) => ReturnType;
      replaceAll: (replacement: string) => ReturnType;
    };
  }
}

export function getSearchState(state: Parameters<typeof searchKey.getState>[0]) {
  return searchKey.getState(state);
}

export const SearchReplace = Extension.create({
  name: "searchReplace",

  addCommands() {
    return {
      setSearch:
        (query, options) =>
        ({ tr, dispatch }) => {
          if (dispatch) tr.setMeta(searchKey, { query, options });
          return true;
        },
      clearSearch:
        () =>
        ({ tr, dispatch }) => {
          if (dispatch) tr.setMeta(searchKey, { query: "", options: { caseSensitive: false, wholeWord: false } });
          return true;
        },
      gotoMatch:
        (direction) =>
        ({ state, tr, dispatch, view }) => {
          const search = searchKey.getState(state);
          if (!search || search.matches.length === 0) return false;
          const head = state.selection.from;
          let index: number;
          if (direction === 1) {
            index = search.matches.findIndex((match) => match.from > head || (match.from === head && search.current !== search.matches.indexOf(match)));
            if (index < 0) index = 0;
            if (search.current >= 0 && search.matches[search.current]?.from === head) index = (search.current + 1) % search.matches.length;
          } else {
            const before = search.matches.map((match, i) => ({ match, i })).filter(({ match }) => match.from < head);
            index = before.length ? before[before.length - 1].i : search.matches.length - 1;
          }
          const match = search.matches[index];
          if (dispatch) {
            tr.setSelection(TextSelection.create(tr.doc, match.from, match.to));
            tr.setMeta(searchKey, { current: index });
            tr.scrollIntoView();
            view?.focus();
          }
          return true;
        },
      replaceCurrent:
        (replacement) =>
        ({ state, tr, dispatch }) => {
          const search = searchKey.getState(state);
          if (!search || search.matches.length === 0) return false;
          const index = search.current >= 0 ? search.current : 0;
          const match = search.matches[index];
          if (dispatch) {
            if (replacement) tr.insertText(replacement, match.from, match.to);
            else tr.delete(match.from, match.to);
            const next = Math.min(index, search.matches.length - 2);
            tr.setMeta(searchKey, { current: next });
          }
          return true;
        },
      replaceAll:
        (replacement) =>
        ({ state, tr, dispatch }) => {
          const search = searchKey.getState(state);
          if (!search || search.matches.length === 0) return false;
          if (dispatch) {
            for (const match of [...search.matches].reverse()) {
              if (replacement) tr.insertText(replacement, match.from, match.to);
              else tr.delete(match.from, match.to);
            }
          }
          return true;
        },
    };
  },

  addProseMirrorPlugins() {
    return [
      new Plugin<SearchState>({
        key: searchKey,
        state: {
          init: (_, state) => ({
            query: "",
            options: { caseSensitive: false, wholeWord: false },
            matches: [],
            current: -1,
            decorations: DecorationSet.create(state.doc, []),
          }),
          apply: (tr, value) => {
            const meta = tr.getMeta(searchKey) as Partial<Pick<SearchState, "query" | "options" | "current">> | undefined;
            if (!meta && !tr.docChanged) return value;
            const query = meta?.query ?? value.query;
            const options = meta?.options ?? value.options;
            const matches = findMatches(tr.doc, query, options);
            let current = meta?.current ?? (meta?.query !== undefined ? -1 : value.current);
            if (current >= matches.length) current = matches.length - 1;
            return { query, options, matches, current, decorations: decorate(tr.doc, matches, current) };
          },
        },
        props: {
          decorations: (state) => searchKey.getState(state)?.decorations,
        },
      }),
    ];
  },
});
