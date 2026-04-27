mod config;
mod db;
mod dbf_reader;
mod grpc;
mod models;
mod types;

use config::AppConfig;
use grpc::GrpcClient;
use types::*;

use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::Mutex;

// Estado compartido: None si aún no hay configuración
type GrpcState = Arc<Mutex<Option<GrpcClient>>>;

// ── Comandos Tauri ─────────────────────────────────────────────

#[tauri::command]
async fn list_almacenes(grpc: State<'_, GrpcState>) -> Result<Vec<AlmacenRecord>, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .list_almacenes()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_documentos(
    grpc: State<'_, GrpcState>,
    params: ListDocumentosParams,
) -> Result<ListDocumentosResult, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .list_documentos(params)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_documento(
    grpc: State<'_, GrpcState>,
    tipodoc: String,
    numdoc: String,
) -> Result<GetDocumentoResult, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .get_documento(tipodoc, numdoc)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_proveedor_nombre(
    grpc: State<'_, GrpcState>,
    numprov: String,
) -> Result<String, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .get_proveedor_nombre(numprov)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn buscar_seguimiento(
    grpc: State<'_, GrpcState>,
    numarts: Vec<String>,
    fecha_desde: String,
    numalm: String,
) -> Result<SeguimientoResult, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .buscar_seguimiento(numarts, fecha_desde, numalm)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_articulos(
    grpc: State<'_, GrpcState>,
    numarts: Vec<String>,
) -> Result<Vec<ArticuloInfo>, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .get_articulos(numarts)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_config_path() -> Option<String> {
    AppConfig::config_path().map(|p| p.display().to_string())
}

#[tauri::command]
fn check_config() -> bool {
    AppConfig::load().is_ok()
}

#[tauri::command]
async fn save_config(
    grpc: State<'_, GrpcState>,
    endpoint: String,
    api_key: String,
) -> Result<(), String> {
    AppConfig::save(&endpoint, &api_key).map_err(|e| e.to_string())?;
    let client = GrpcClient::new(&endpoint, &api_key).map_err(|e| e.to_string())?;
    *grpc.lock().await = Some(client);
    Ok(())
}

// ── Comandos DBF ───────────────────────────────────────────────

#[derive(serde::Serialize)]
struct DbfPaths {
    dbf_arts: Option<String>,
    dbf_unidades: Option<String>,
}

#[tauri::command]
fn get_dbf_paths() -> DbfPaths {
    let cfg = AppConfig::load().ok();
    DbfPaths {
        dbf_arts: cfg.as_ref().and_then(|c| c.dbf_arts.clone()),
        dbf_unidades: cfg.as_ref().and_then(|c| c.dbf_unidades.clone()),
    }
}

#[tauri::command]
fn save_dbf_arts(path: String) -> Result<(), String> {
    AppConfig::update_field("dbf_arts", &path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_dbf_unidades(path: String) -> Result<(), String> {
    AppConfig::update_field("dbf_unidades", &path).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct FraccionesInitData {
    fracciones: Vec<ArticuloFracciones>,
    articulos: Vec<ArticuloSearchResult>,
    etiquetas: Vec<Etiqueta>,
}

#[tauri::command]
fn get_fracciones_init_data() -> Result<FraccionesInitData, String> {
    use std::collections::HashMap;
    use std::path::Path;

    let cfg = AppConfig::load().ok();

    let arts_path = cfg
        .as_ref()
        .and_then(|c| c.dbf_arts.as_deref())
        .ok_or_else(|| "Archivo de artículos no configurado".to_string())?;

    let unidades_path = cfg
        .as_ref()
        .and_then(|c| c.dbf_unidades.as_deref())
        .ok_or_else(|| "Archivo de fracciones no configurado".to_string())?;

    let articulos_raw = dbf_reader::read_articulos(Path::new(arts_path))
        .map_err(|e| e.to_string())?;
    let unidades = dbf_reader::read_unidades(Path::new(unidades_path))
        .map_err(|e| e.to_string())?;

    let arts_map: HashMap<String, &models::Articulo> = articulos_raw
        .iter()
        .filter(|a| !a.deleted_in_dbf)
        .map(|a| (a.numart.clone(), a))
        .collect();

    let pairings = db::get_all_pairings().unwrap_or_default();
    let pairing_etiquetas = db::get_all_pairing_etiquetas().unwrap_or_default();

    let mut fracciones_map: HashMap<String, Vec<FraccionRecord>> = HashMap::new();
    for u in &unidades {
        if u.deleted_in_dbf {
            continue;
        }
        let pareado = pairings
            .get(&(u.numart.clone(), u.unidad.clone()))
            .and_then(|destino| arts_map.get(destino.as_str()))
            .map(|art| ArticuloPareado {
                numart: art.numart.clone(),
                desc: art.desc.clone(),
                unidad: art.unidad.clone(),
                precio1: art.precio1,
                precio2: art.precio2,
                precio3: art.precio3,
                precio4: art.precio4,
                precio5: art.precio5,
            });

        let etiquetas = pairing_etiquetas
            .get(&(u.numart.clone(), u.unidad.clone()))
            .cloned()
            .unwrap_or_default();

        fracciones_map
            .entry(u.numart.clone())
            .or_default()
            .push(FraccionRecord {
                unidad: u.unidad.clone(),
                equiv1: u.equiv1,
                equiv2: u.equiv2,
                precio1: u.precio1,
                precio2: u.precio2,
                precio3: u.precio3,
                precio4: u.precio4,
                precio5: u.precio5,
                pareado,
                etiquetas,
            });
    }

    let mut fracciones: Vec<ArticuloFracciones> = fracciones_map
        .into_iter()
        .map(|(numart, fracs)| {
            let art = arts_map.get(&numart);
            ArticuloFracciones {
                numart: numart.clone(),
                desc: art.map(|a| a.desc.clone()).unwrap_or_default(),
                unidad_base: art.map(|a| a.unidad.clone()).unwrap_or_default(),
                fracciones: fracs,
            }
        })
        .collect();
    fracciones.sort_by(|a, b| a.desc.cmp(&b.desc));

    let mut articulos: Vec<ArticuloSearchResult> = articulos_raw
        .into_iter()
        .filter(|a| !a.deleted_in_dbf)
        .map(|a| ArticuloSearchResult {
            numart: a.numart,
            desc: a.desc,
            unidad: a.unidad,
        })
        .collect();
    articulos.sort_by(|a, b| a.desc.cmp(&b.desc));

    let etiquetas = db::get_all_etiquetas().unwrap_or_default();

    Ok(FraccionesInitData { fracciones, articulos, etiquetas })
}

#[tauri::command]
fn save_fraccion_pairing(
    numart_origen: String,
    unidad_fraccion: String,
    numart_destino: String,
) -> Result<(), String> {
    db::upsert_pairing(&numart_origen, &unidad_fraccion, &numart_destino)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_fraccion_pairing(
    numart_origen: String,
    unidad_fraccion: String,
) -> Result<(), String> {
    db::delete_pairing(&numart_origen, &unidad_fraccion)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_etiquetas() -> Result<Vec<Etiqueta>, String> {
    db::get_all_etiquetas().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_etiqueta(nombre: String, color: String) -> Result<Etiqueta, String> {
    db::create_etiqueta(&nombre, &color).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_etiqueta(id: i64, nombre: String, color: String) -> Result<(), String> {
    db::update_etiqueta(id, &nombre, &color).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_etiqueta(id: i64) -> Result<(), String> {
    db::delete_etiqueta(id).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_emparejamiento_etiquetas(
    numart_origen: String,
    unidad_fraccion: String,
    etiqueta_ids: Vec<i64>,
) -> Result<(), String> {
    db::set_pairing_etiquetas(&numart_origen, &unidad_fraccion, &etiqueta_ids)
        .map_err(|e| e.to_string())
}

// ── Entry point ────────────────────────────────────────────────

#[tauri::command]
async fn init_client(grpc: State<'_, GrpcState>) -> Result<bool, String> {
    match AppConfig::load() {
        Ok(config) => {
            let client = GrpcClient::new(&config.grpc_endpoint, &config.api_key)
                .map_err(|e| e.to_string())?;
            *grpc.lock().await = Some(client);
            Ok(true)
        }
        Err(_) => Ok(false),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("lufal_ordenes_tauri_lib=debug".parse().unwrap()),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // State starts empty; the frontend calls init_client on startup.
            // GrpcClient::new() requires an active Tokio reactor (hyper-util 0.1.20+),
            // so it cannot be created here in the sync setup() callback.
            let grpc_state: GrpcState = Arc::new(Mutex::new(None));
            app.manage(grpc_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_almacenes,
            list_documentos,
            get_documento,
            get_proveedor_nombre,
            get_articulos,
            buscar_seguimiento,
            get_config_path,
            check_config,
            save_config,
            init_client,
            get_dbf_paths,
            save_dbf_arts,
            save_dbf_unidades,
            get_fracciones_init_data,
            save_fraccion_pairing,
            delete_fraccion_pairing,
            get_etiquetas,
            create_etiqueta,
            update_etiqueta,
            delete_etiqueta,
            set_emparejamiento_etiquetas,
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar la aplicación Tauri");
}
