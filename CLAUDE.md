# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Desarrollo
npm run dev              # Vite dev server (frontend en localhost:1420)
npm run tauri dev        # App Tauri completa en modo desarrollo

# Verificación
npm run check            # svelte-check + TypeScript
cargo check              # Solo backend Rust (dentro de src-tauri/)

# Build / Deploy
npm run build            # Compila frontend → dist/
npm run tauri build -- --bundles nsis   # Instalador NSIS firmado
npm run deploy           # Auto-versiona, compila, firma y sube al servidor
```

El deploy (`scripts/deploy.py`) calcula la versión desde git tags, construye con firma (`TAURI_SIGNING_PRIVATE_KEY`), genera `latest.json` y sube por SSH al alias `cloud-api:/var/www/updates`.

## Arquitectura

**Lufal Auxiliar Desktop** es una app de gestión (inventario, órdenes, etiquetas, estadísticas) para Pinturas Lufal. Stack: Tauri 2 + Svelte 5 + Rust.

### Capas de datos

| Fuente | Tecnología | Qué contiene |
|--------|-----------|--------------|
| Servidor remoto | gRPC / tonic (`VfpSyncService`) | Documentos, artículos, almacenes, proveedores, CxC |
| Archivos locales | DBF (dBASE) vía `dbf_reader.rs` | Docum, Arts, Minv, CxC por almacén (cache con invalidación por mtime + año) |
| Base local | SQLite (`db.rs`) | Pairings de fracciones, etiquetas, seguimientos — datos generados por el usuario, no se sincronizan |

### Flujo IPC

```
Svelte (invoke) → Tauri handler (lib.rs) → GrpcClient (grpc.rs) → VfpSyncService
                                         ↘ dbf_reader / db / print (directo)
```

- El frontend **nunca** llama gRPC directamente; todo pasa por `invoke()`.
- Los tipos se definen en `src/lib/types.ts` y se reflejan en `src-tauri/src/types.rs`.
- Las funciones de Tauri están en `src/lib/grpc.ts` (son delgadas, solo invocan).

### Estado compartido en Rust

```rust
type GrpcState = Arc<Mutex<Option<GrpcClient>>>;
```

`None` mientras no hay configuración guardada; `Some(client)` luego de `save_config`. El cliente usa `connect_lazy()` — la conexión TCP/TLS real ocurre en el primer RPC.

### Configuración por almacén

`config.toml` (en `%APPDATA%\lufal-auxiliar-desktop\`) contiene un array `sucursales` con `numalm`, `letra` y `dbf_path`. El `default_numalm` marca el almacén activo. La ConfigView permite editar todo esto y llama `save_config` que reinicializa el cliente gRPC.

### Impresión de etiquetas (print.rs)

Flujo completamente en Rust vía Win32 GDI (sin subprocesos):

1. Frontend renderiza la etiqueta en `<canvas>` con jsbarcode → base64 PNG
2. Llama `invoke('print_etiquetas', { labels, heightMm, printerName })`
3. Rust: `OpenPrinterW` → `DocumentPropertiesW` (DEVMODE portrait) → `CreateDCW` → `StretchDIBits`
4. DEVMODE: `dmPaperWidth = 620` (62 mm), `dmPaperLength = height_mm × 10`

`list_printers` usa `EnumPrintersW` directamente (sin PowerShell).

### Proto / gRPC

El archivo `src-tauri/proto/vfpsync.proto` define `VfpSyncService` con ~40 RPCs. `tonic-build` lo compila en `build.rs`. Los campos `fecha` son strings ISO 8601 (`YYYY-MM-DD`); `fechahora` es `YYYYMMDDHHMMSS` sin separadores.

### Actualización automática

`tauri-plugin-updater` verifica `https://central.pinturaslufal.com/updates/latest.json`. El deploy genera y sube ese manifiesto junto con el instalador firmado.
