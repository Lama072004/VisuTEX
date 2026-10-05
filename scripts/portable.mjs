#!/usr/bin/env node
// Stellt nach `npm run exe` eine portable Testversion zusammen:
//   src-tauri/target/release/VisuTeX-portable/
//     VisuTeX(.exe)      Programm (Release, statisch gelinkt)
//     resources/         TeX-Bundle, Skizzen-Symbole, Add-ons
//     licenses/          Lizenzen und Drittanbieter-Hinweise
// Der Ordner läuft ohne Installation (Windows: WebView2 muss vorhanden sein – unter
// Windows 10/11 ist es vorinstalliert; der Installer bringt es zusätzlich mit).
//
// Optionen:
//   --target <triple>   Build mit `tauri build --target …` (z. B. in der CI)
//   --zip               zusätzlich VisuTeX_<Version>_x64-portable.zip daneben erzeugen
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
cpSync(join(root, "src-tauri", "resources"), join(target, "resources"), { recursive: true });
cpSync(join(root, "LICENSE"), join(target, "licenses", "LICENSE.txt"));
cpSync(join(root, "THIRD_PARTY_NOTICES.md"), join(target, "licenses", "THIRD_PARTY_NOTICES.md"));
cpSync(join(root, "licenses", "GPL-2.0.txt"), join(target, "licenses", "GPL-2.0.txt"));

const megabytes = (path) => (statSync(path).size / 1024 / 1024).toFixed(1);
console.log(`✔ Portable Version: ${target}`);
console.log(`  Programm ${megabytes(binary)} MB · TeX-Bundle ${megabytes(join(target, "resources", "tex-bundle.zip"))} MB`);
console.log(`  Starten: ${join(target, windows ? "VisuTeX.exe" : "VisuTeX")}`);

if (args.includes("--zip")) {
  const { version } = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const zip = join(release, `VisuTeX_${version}_x64-portable.zip`);
  rmSync(zip, { force: true });
  if (windows) {
    execFileSync("powershell", ["-NoProfile", "-Command", `Compress-Archive -Path '${target}' -DestinationPath '${zip}'`], { stdio: "inherit" });
  } else {
    execFileSync("zip", ["-qr", zip, "VisuTeX-portable"], { cwd: release, stdio: "inherit" });
  }
  console.log(`✔ ZIP: ${zip} (${megabytes(zip)} MB)`);
}
