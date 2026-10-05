#!/usr/bin/env node
// Einrichtungsassistent für VisuTeX-Entwicklung unter Windows, Linux und macOS.
//
//   npm run setup            prüft alles und bietet fehlende Teile zur Installation an
//                            (fragt jeweils nach – auch nach dem Installationsort)
//   npm run doctor           nur prüfen, nichts installieren
//   npm run setup -- --yes   alle Vorschläge mit Standardwerten übernehmen (z. B. für CI)
import { execSync, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { arch, platform } from "node:os";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline/promises";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const tauriDir = join(root, "src-tauri");
const checkOnly = process.argv.includes("--check");
const assumeYes = process.argv.includes("--yes");
const os = platform();
const cpu = arch();
const triplet =
  os === "win32" ? (cpu === "arm64" ? "arm64-windows-static" : "x64-windows-static")
  : os === "darwin" ? (cpu === "arm64" ? "arm64-osx" : "x64-osx")
  : cpu === "arm64" ? "arm64-linux" : "x64-linux";

const rl = checkOnly || assumeYes ? null : createInterface({ input: process.stdin, output: process.stdout });

function quiet(command) {
  try {
    return execSync(command, { stdio: ["ignore", "pipe", "ignore"] }).toString().trim();
  } catch {
    return null;
  }
}

function runVisible(command, args, options = {}) {
  console.log(`\n  $ ${[command, ...args].join(" ")}`);
  const result = spawnSync(command, args, { stdio: "inherit", shell: os === "win32", ...options });
  return result.status === 0;
}

async function ask(question, fallback = "j") {
  if (assumeYes) return fallback;
  if (!rl) return "n";
  const answer = (await rl.question(`  ? ${question} `)).trim();
  return answer || fallback;
}

async function confirm(question) {
  return /^(j|ja|y|yes)$/i.test(await ask(`${question} [J/n]`, "j"));
}

/** Rechnerlokale Cargo-Konfiguration (src-tauri/.cargo/config.toml, nicht versioniert) ergänzen. */
function setLocalEnv(key, tomlValue) {
  const dir = join(tauriDir, ".cargo");
  const file = join(dir, "config.toml");
  const entries = new Map();
  if (existsSync(file)) {
    for (const line of readFileSync(file, "utf8").split(/\r?\n/)) {
      const match = line.match(/^([A-Z_]+)\s*=\s*(.+)$/);
      if (match) entries.set(match[1], match[2]);
    }
  }
  entries.set(key, tomlValue);
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    file,
    `# Nur dieser Rechner (nicht versioniert) – gepflegt von scripts/setup.mjs.\n[env]\n${[...entries].map(([name, value]) => `${name} = ${value}`).join("\n")}\n`,
  );
}
let missing = 0;
function report(ok, name, hint) {
  console.log(`${ok ? "✔" : "✖"} ${name}`);
  if (!ok) {
    missing += 1;
    if (hint) console.log(`    → ${hint}`);
  }
  return ok;
}

console.log(`VisuTeX – Einrichtung (${os}/${cpu})\n`);

// 1. Node.js
const nodeMajor = Number(process.versions.node.split(".")[0]);
report(nodeMajor >= 20, `Node.js ${process.versions.node}`, "Node.js 20 oder neuer installieren: https://nodejs.org");

// 2. Rust
const cargo = quiet("cargo --version");
if (!report(Boolean(cargo), cargo ?? "Rust/Cargo", "Rust installieren: https://rustup.rs (danach Terminal neu öffnen)")) {
  console.log("\nOhne Rust kann nicht weiter geprüft werden.");
  process.exit(1);
}

// 3. Systempakete (C-Compiler, WebView-Entwicklungsdateien, Werkzeuge für vcpkg)
if (os === "linux") {
  const packages = [];
  if (quiet("pkg-config --exists webkit2gtk-4.1 libsoup-3.0 && echo ok") !== "ok") packages.push("libwebkit2gtk-4.1-dev", "libsoup-3.0-dev", "librsvg2-dev", "patchelf");
  const tools = { autoconf: "autoconf autoconf-archive", automake: "automake", libtool: "libtool", bison: "bison", gperf: "gperf", python3: "python3 python3-venv", cc: "build-essential", "pkg-config": "pkg-config" };
  for (const [tool, pkg] of Object.entries(tools)) if (!quiet(`command -v ${tool}`)) packages.push(...pkg.split(" "));
  if (!report(packages.length === 0, "Linux-Systempakete", `sudo apt install ${packages.join(" ")}`) && !checkOnly) {
    if (quiet("command -v apt-get") && (await confirm("Jetzt mit apt installieren (sudo-Passwort nötig)?"))) {
      runVisible("sudo", ["apt-get", "install", "-y", ...packages]);
    } else {
      console.log("    Bitte mit dem Paketmanager deiner Distribution installieren (Fedora: webkit2gtk4.1-devel …).");
    }
  }
}
if (os === "darwin") {
  if (!report(Boolean(quiet("xcode-select -p")), "Xcode Command Line Tools", "xcode-select --install") && !checkOnly) {
    if (await confirm("Xcode Command Line Tools jetzt installieren?")) runVisible("xcode-select", ["--install"]);
  }
  const tools = ["autoconf", "automake", "libtool", "pkg-config"].filter((tool) => !quiet(`command -v ${tool}`));
  if (!report(tools.length === 0, "Build-Werkzeuge (Homebrew)", `brew install ${tools.join(" ")} autoconf-archive`) && !checkOnly) {
    if (quiet("command -v brew") && (await confirm("Mit Homebrew installieren?"))) runVisible("brew", ["install", ...tools, "autoconf-archive"]);
    else console.log("    Homebrew installieren: https://brew.sh");
  }
}
if (os === "win32") {
  const vswhere = "C:\\Program Files (x86)\\Microsoft Visual Studio\\Installer\\vswhere.exe";
  const msvc = existsSync(vswhere) ? quiet(`"${vswhere}" -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) : null;
  if (!report(Boolean(msvc), "Visual Studio C++-Build-Tools", "winget install Microsoft.VisualStudio.2022.BuildTools (Workload „Desktopentwicklung mit C++“)") && !checkOnly) {
    if (quiet("where winget") && (await confirm("Mit winget installieren (der Installer fragt nach dem Zielordner)?"))) {
      runVisible("winget", ["install", "--id", "Microsoft.VisualStudio.2022.BuildTools", "--override", "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended", "-e", "-i"]);
    }
  }
}

// 4. npm-Abhängigkeiten
if (!report(existsSync(join(root, "node_modules", "@tauri-apps", "cli")), "npm-Abhängigkeiten", "npm install") && !checkOnly) {
  if (await confirm("npm install ausführen?")) runVisible("npm", ["install"], { cwd: root });
}

// 5. cargo-vcpkg
if (!report(Boolean(quiet("cargo vcpkg --help")), "cargo-vcpkg", "cargo install cargo-vcpkg") && !checkOnly) {
  if (await confirm("cargo-vcpkg installieren?")) runVisible("cargo", ["install", "cargo-vcpkg", "--locked"]);
}

// 5b. Windows: Umgebungsvariablen, die dynamische vcpkg-Bibliotheken erzwingen (Installer bräuchte DLLs)
if (os === "win32" && process.env.TECTONIC_DEP_BACKEND !== "pkg-config") {
  const dynamicTriplet = process.env.VCPKGRS_TRIPLET && !/static/.test(process.env.VCPKGRS_TRIPLET);
  const localConfig = join(tauriDir, ".cargo", "config.toml");
  const overridden = existsSync(localConfig) && /VCPKGRS_TRIPLET\s*=.*static.*force\s*=\s*true/.test(readFileSync(localConfig, "utf8"));
  const conflict = (dynamicTriplet || process.env.VCPKGRS_DYNAMIC) && !overridden;
  if (!report(!conflict, "Statisches Linken (keine DLLs im Installer)", `Umgebungsvariablen VCPKGRS_TRIPLET=${process.env.VCPKGRS_TRIPLET ?? ""} / VCPKGRS_DYNAMIC=${process.env.VCPKGRS_DYNAMIC ?? ""} erzwingen dynamische Bibliotheken.`) && !checkOnly) {
    if (await confirm("Für dieses Projekt statisch linken (nur src-tauri/.cargo/config.toml, andere Projekte bleiben unverändert)?")) {
      setLocalEnv("VCPKGRS_TRIPLET", `{ value = "${triplet}", force = true }`);
      console.log("    Eingetragen. Danach einmal neu bauen: cd src-tauri && cargo clean && cargo build");
    }
  }
}
// 6. Native TeX-Bibliotheken (vcpkg, statisch)
const envRoot = process.env.VCPKG_ROOT;
const defaultRoot = envRoot || join(tauriDir, "target", "vcpkg");
const libsIn = (vcpkgRoot) => {
  const dir = join(vcpkgRoot, "installed", triplet, "lib");
  const names = existsSync(dir) ? readdirSync(dir).join(" ").toLowerCase() : "";
  return ["harfbuzz", "icu", "freetype"].every((name) => names.includes(name)) && (os === "darwin" || names.includes("fontconfig"));
};
if (process.env.TECTONIC_DEP_BACKEND === "pkg-config") {
  const ok = quiet("pkg-config --exists harfbuzz icu-uc freetype2 graphite2 && echo ok") === "ok";
  report(ok, "TeX-Bibliotheken (pkg-config)", "Systempakete installieren (docs/ENTWICKLUNG.md) oder TECTONIC_DEP_BACKEND entfernen");
} else if (!report(libsIn(defaultRoot), `TeX-Bibliotheken (vcpkg ${triplet})`, `cd src-tauri && cargo vcpkg build`) && !checkOnly) {
  console.log(`    vcpkg baut ICU, HarfBuzz, FreeType${os === "darwin" ? "" : ", Fontconfig"} statisch (einmalig, ca. 20–40 Minuten, ~3 GB).`);
  const answer = await ask(`Wo soll vcpkg installiert werden? [${defaultRoot}]`, defaultRoot);
  const target = resolve(answer);
  if (await confirm(`vcpkg nach „${target}“ installieren und bauen?`)) {
    mkdirSync(target, { recursive: true });
    const isDefault = target === resolve(join(tauriDir, "target", "vcpkg")) && !envRoot;
    if (!isDefault) {
      // Rechnerspezifisch, daher in einer nicht versionierten Cargo-Konfiguration.
      setLocalEnv("VCPKG_ROOT", JSON.stringify(target));
      console.log("    VCPKG_ROOT wurde in src-tauri/.cargo/config.toml (nur dieser Rechner) eingetragen.");
    }
    runVisible("cargo", ["vcpkg", "-v", "build"], { cwd: tauriDir, env: { ...process.env, ...(isDefault ? {} : { VCPKG_ROOT: target }) } });
  }
}

// 7. Mitgeliefertes TeX-Bundle
const bundle = join(tauriDir, "resources", "tex-bundle.zip");
if (!report(existsSync(bundle), "Mitgeliefertes TeX-Bundle", "cd src-tauri && cargo run --features dev-tools --bin build-tex-bundle") && !checkOnly) {
  if (await confirm("TeX-Bundle jetzt erzeugen (lädt einmalig TeX-Pakete aus dem Internet)?")) {
    runVisible("cargo", ["run", "--features", "dev-tools", "--bin", "build-tex-bundle"], { cwd: tauriDir });
  }
}

rl?.close();
console.log(missing ? `\n${missing} Punkt(e) offen.${checkOnly ? " Mit „npm run setup“ einrichten." : " Assistent erneut starten, um zu prüfen."}` : "\nAlles bereit: npm run tauri dev");
process.exit(missing && checkOnly ? 1 : 0);
