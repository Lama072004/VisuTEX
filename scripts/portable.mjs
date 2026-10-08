#!/usr/bin/env node
// Stellt nach `npm run exe` eine portable Version zusammen:
//   src-tauri/target/release/VisuTeX-portable/
//     VisuTeX(.exe)      Programm – enthält alles (TeX-Bundle, Skizzen-Symbole, Add-ons) und läuft
//                        auch allein, z. B. auf den Desktop kopiert
//     licenses/          Lizenzen und Drittanbieter-Hinweise (zum Weitergeben)
// Läuft ohne Installation (Windows: WebView2 muss vorhanden sein – unter Windows 10/11 ist es
// vorinstalliert; der Installer bringt es zusätzlich mit).
//
// Optionen:
//   --target <triple>   Build mit `tauri build --target …` (z. B. in der CI)
//   --zip               zusätzlich VisuTeX_<Version>_x64-portable.zip daneben erzeugen
//   --exe               zusätzlich VisuTeX_<Version>_windows_x64.exe (einzelne Programmdatei für
//                       Releases; die Update-Suche der App lädt genau diese Datei)
// Version: VISUTEX_VERSION (in der CI aus dem Tag), sonst package.json.
import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, statSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const args = process.argv.slice(2);
const targetIndex = args.indexOf("--target");
const triple = targetIndex >= 0 ? args[targetIndex + 1] : null;
const release = triple ? join(root, "src-tauri", "target", triple, "release") : join(root, "src-tauri", "target", "release");
const windows = process.platform === "win32";
const binary = join(release, windows ? "visutex.exe" : "visutex");

if (!existsSync(binary)) {
  console.error(`✖ ${binary} fehlt – zuerst „npm run exe“ ausführen.`);
  process.exit(1);
}

const target = join(release, "VisuTeX-portable");
rmSync(target, { recursive: true, force: true });
mkdirSync(join(target, "licenses"), { recursive: true });

cpSync(binary, join(target, windows ? "VisuTeX.exe" : "VisuTeX"));
cpSync(join(root, "LICENSE"), join(target, "licenses", "LICENSE.txt"));
cpSync(join(root, "THIRD_PARTY_NOTICES.md"), join(target, "licenses", "THIRD_PARTY_NOTICES.md"));
cpSync(join(root, "licenses", "GPL-2.0.txt"), join(target, "licenses", "GPL-2.0.txt"));

const megabytes = (path) => (statSync(path).size / 1024 / 1024).toFixed(1);
console.log(`✔ Portable Version: ${target}`);
console.log(`  Programm ${megabytes(binary)} MB (enthält TeX-Bundle, Skizzen-Symbole und Add-ons)`);
console.log(`  Starten: ${join(target, windows ? "VisuTeX.exe" : "VisuTeX")}`);

const version = process.env.VISUTEX_VERSION?.trim() || JSON.parse(readFileSync(join(root, "package.json"), "utf8")).version;

if (args.includes("--exe") && windows) {
  const exe = join(release, `VisuTeX_${version}_windows_x64.exe`);
  cpSync(binary, exe);
  console.log(`✔ Einzelne Programmdatei: ${exe}`);
}

if (args.includes("--zip")) {
  const zip = join(release, `VisuTeX_${version}_x64-portable.zip`);
  rmSync(zip, { force: true });
  if (windows) {
    execFileSync("powershell", ["-NoProfile", "-Command", `Compress-Archive -Path '${target}' -DestinationPath '${zip}'`], { stdio: "inherit" });
  } else {
    execFileSync("zip", ["-qr", zip, "VisuTeX-portable"], { cwd: release, stdio: "inherit" });
  }
  console.log(`✔ ZIP: ${zip} (${megabytes(zip)} MB)`);
}
