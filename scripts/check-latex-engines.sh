#!/bin/bash
# Prüft, ob ein exportiertes Dokument mit pdfLaTeX, XeLaTeX und LuaLaTeX (inkl. BibTeX) kompiliert.
# Aufruf: scripts/check-latex-engines.sh <ordner> <dateiname-ohne-.tex>
# Beispiel nach `cargo test --test core_roundtrip`: scripts/check-latex-engines.sh src-tauri/target/visutex-roundtrip doc
# usage: compile-all.sh <dir> <file-stem>
cd "$1"; stem="$2"; status=0
for engine in pdflatex xelatex lualatex; do
  rm -f "$stem".aux "$stem".bbl "$stem".blg "$stem".pdf
  ok=1
  timeout 300 $engine -interaction=nonstopmode -halt-on-error "$stem".tex > /dev/null 2>&1 || ok=0
  if [ $ok = 1 ] && grep -q '\\bibdata' "$stem".aux 2>/dev/null; then bibtex "$stem" > /dev/null 2>&1 || { echo "  bibtex failed"; tail -5 "$stem".blg; }; fi
  [ $ok = 1 ] && { timeout 300 $engine -interaction=nonstopmode -halt-on-error "$stem".tex > /dev/null 2>&1 || ok=0; }
  [ $ok = 1 ] && { timeout 300 $engine -interaction=nonstopmode -halt-on-error "$stem".tex > /dev/null 2>&1 || ok=0; }
  if [ $ok = 1 ]; then
    pages=$(pdfinfo "$stem".pdf 2>/dev/null | awk '/^Pages/{print $2}')
    warn=$(grep -c "LaTeX Warning\|Package .* Warning" "$stem".log)
    undef=$(grep -c "undefined" "$stem".log)
    echo "$engine: OK ($pages pages, $warn warnings, $undef 'undefined' mentions)"
    cp "$stem".pdf "$stem-$engine.pdf"
  else
    status=1
    echo "$engine: FAILED"; grep -n "^!" -A4 "$stem".log | head -20
  fi
done
exit $status
