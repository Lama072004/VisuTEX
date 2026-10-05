use std::env;
use std::path::PathBuf;

fn main() {
    link_vcpkg_statically_on_linux();
    tauri_build::build()
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
