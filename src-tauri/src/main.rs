// Evita ventana de consola en Windows (release) — NO ELIMINAR
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    lufal_ordenes_tauri_lib::run()
}
