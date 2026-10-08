use std::env;
use std::path::PathBuf;

fn main() {
    link_vcpkg_statically_on_linux();
    list_embedded_addons();
    embed_tex_bundle();
    tauri_build::build()
}

/// TeX-Bundle (≈ 66 MB) per Assembler-`.incbin` in das Programm einbetten. Ein `include_bytes!`
/// ginge beim Release-Build (LTO) komplett durch den LLVM-Optimierer und verlängerte den Build
/// um ein Vielfaches; `.incbin` kopiert die Datei erst beim Assemblieren. `main.rs` liest die
/// Bytes zwischen den Symbolen `visutex_tex_bundle_start` und `visutex_tex_bundle_end`.
fn embed_tex_bundle() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let bundle = manifest.join("resources").join("tex-bundle.zip");
    println!("cargo:rerun-if-changed={}", bundle.display());
    let path = bundle.to_string_lossy().replace('\\', "/");
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let (section, prefix) = match target_os.as_str() {
        "windows" => (".section .rdata,\"dr\"", ""),
        "macos" | "ios" => (".section __TEXT,__const", "_"),
        _ => (".section .rodata", ""),
    };
    let lines = [
        section.to_string(),
        ".balign 16".to_string(),
        format!(".globl {prefix}visutex_tex_bundle_start"),
        format!("{prefix}visutex_tex_bundle_start:"),
        format!(".incbin \"{path}\""),
        format!(".globl {prefix}visutex_tex_bundle_end"),
        format!("{prefix}visutex_tex_bundle_end:"),
    ];
    let template: String = lines
        .iter()
        .map(|line| format!("    {line:?},\n"))
        .collect();
    let code = format!(
        "// Von build.rs erzeugt: TeX-Bundle als Assembler-Daten.\ncore::arch::global_asm!(\n{template});\n"
    );
    let out = PathBuf::from(env::var("OUT_DIR").unwrap_or_default()).join("embedded_bundle.rs");
    if std::fs::read_to_string(&out).ok().as_deref() != Some(code.as_str()) {
        let _ = std::fs::write(out, code);
    }
}

/// Liste der mitgelieferten Add-on-Dateien für `main.rs` (`include_bytes!` je Datei), damit die
/// EXE sie eingebettet enthält – kein Ordner `resources` neben dem Programm nötig.
fn list_embedded_addons() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let root = manifest.join("resources").join("addons");
    println!("cargo:rerun-if-changed={}", root.display());
    let mut files = Vec::new();
    collect_files(&root, &root, &mut files);
    files.sort();
    let entries: String = files
        .iter()
        .map(|(relative, absolute)| format!("    ({relative:?}, include_bytes!({absolute:?})),\n"))
        .collect();
    let code = format!(
        "/// Von build.rs erzeugt: mitgelieferte Add-ons (relativer Pfad, Inhalt).\npub static EMBEDDED_ADDONS: &[(&str, &[u8])] = &[\n{entries}];\n"
    );
    let out = PathBuf::from(env::var("OUT_DIR").unwrap_or_default()).join("embedded_addons.rs");
    if std::fs::read_to_string(&out).ok().as_deref() != Some(code.as_str()) {
        let _ = std::fs::write(out, code);
    }
}

fn collect_files(root: &std::path::Path, dir: &std::path::Path, files: &mut Vec<(String, String)>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else if let Ok(relative) = path.strip_prefix(root) {
            let relative = relative.to_string_lossy().replace('\\', "/");
            files.push((relative, path.to_string_lossy().into_owned()));
        }
    }
}

/// Linux: `vcpkg-rs` meldet die C-Bibliotheken der TeX-Engine ohne „static“ (z. B.
/// `-lgraphite2`). Liegt dieselbe Bibliothek zusätzlich als System-`.so` vor (WebKitGTK
/// bringt graphite2, ICU, FreeType, Fontconfig mit), nimmt der Linker die Systemversion:
/// - Tectonic deklariert graphite2 als „hidden“ (statisch erwartet) → Linkfehler
///   „undefined hidden symbol: gr_…“;
/// - ICU/FreeType/Fontconfig aus dem System binden das Programm an eine Distribution.
///
/// Deshalb werden die statischen vcpkg-Archive hier ausdrücklich eingebunden. rust-lld
/// (Standard-Linker unter Linux) löst damit alle Verweise statisch auf, bevor die
/// System-Bibliotheken von GTK/WebKit in der Befehlszeile folgen.
fn link_vcpkg_statically_on_linux() {
    for name in [
        "TECTONIC_DEP_BACKEND",
        "VCPKG_ROOT",
        "VCPKGRS_TRIPLET",
        "CARGO_TARGET_DIR",
        "VISUTEX_VERSION",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    let target = env::var("TARGET").unwrap_or_default();
    if !target.contains("linux") || env::var("TECTONIC_DEP_BACKEND").as_deref() != Ok("vcpkg") {
        return;
    }
    let triplet = env::var("VCPKGRS_TRIPLET").unwrap_or_else(|_| {
        if target.starts_with("aarch64") {
            "arm64-linux".into()
        } else {
            "x64-linux".into()
        }
    });
    // Gleiche Suche wie vcpkg-rs: VCPKG_ROOT, sonst der von `cargo vcpkg build` angelegte Baum.
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let roots = [
        env::var_os("VCPKG_ROOT").map(PathBuf::from),
        env::var_os("CARGO_TARGET_DIR").map(|dir| PathBuf::from(dir).join("vcpkg")),
        Some(manifest.join("target").join("vcpkg")),
    ];
    let Some(lib_dir) = roots
        .into_iter()
        .flatten()
        .map(|root| root.join("installed").join(&triplet).join("lib"))
        .find(|dir| dir.join("libgraphite2.a").is_file())
    else {
        println!("cargo:warning=vcpkg-Bibliotheken für {triplet} nicht gefunden – statisches Linken übersprungen");
        return;
    };
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // Reihenfolge: Abhängige vor ihren Abhängigkeiten. zlib fehlt bewusst: libz-sys bindet es selbst ein.
    for name in [
        "graphite2",
        "fontconfig",
        "freetype",
        "expat",
        "uuid",
        "png16",
        "bz2",
        "brotlidec",
        "brotlienc",
        "brotlicommon",
        "icui18n",
        "icuuc",
        "icudata",
    ] {
        if lib_dir.join(format!("lib{name}.a")).is_file() {
            println!("cargo:rustc-link-lib=static={name}");
        }
    }
}
