// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// Alles, was die App braucht, steckt in der EXE: TeX-Bundle und mitgelieferte Add-ons werden hier
// eingebettet (nur im Programm, nicht in der Bibliothek – Tests bleiben klein) und beim ersten
// Bedarf in den App-Cache entpackt. Das Bundle bindet der Assembler ein (`.incbin`, siehe build.rs).
include!(concat!(env!("OUT_DIR"), "/embedded_addons.rs"));
include!(concat!(env!("OUT_DIR"), "/embedded_bundle.rs"));

extern "C" {
    static visutex_tex_bundle_start: u8;
    static visutex_tex_bundle_end: u8;
}

fn tex_bundle() -> &'static [u8] {
    // SAFETY: Beide Symbole begrenzen die per `.incbin` eingebettete, unveränderliche Datei.
    unsafe {
        let start = std::ptr::addr_of!(visutex_tex_bundle_start);
        let end = std::ptr::addr_of!(visutex_tex_bundle_end);
        std::slice::from_raw_parts(start, end as usize - start as usize)
    }
}

fn main() {
    visutex_lib::run_with(visutex_lib::embedded::EmbeddedAssets {
        tex_bundle: tex_bundle(),
        addons: EMBEDDED_ADDONS,
    })
}
