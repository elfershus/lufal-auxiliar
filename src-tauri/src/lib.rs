mod config;
mod db;
mod dbf_reader;
mod grpc;
mod models;
mod print;
mod types;
mod xlsx;

use config::AppConfig;
use grpc::GrpcClient;
use types::*;

use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::Mutex;

// Estado compartido: None si aún no hay configuración
type GrpcState = Arc<Mutex<Option<GrpcClient>>>;

// Caché de documentos DBF — invalida cuando cambia el mtime o el año solicitado
struct DocumCacheEntry {
    path: String,
    mtime: std::time::SystemTime,
    anio: i32,
    docs: Vec<models::Documento>,
}
struct DocumState(std::sync::Mutex<Option<DocumCacheEntry>>);

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
async fn list_articulos_etiqueta(
    grpc: State<'_, GrpcState>,
    q: Option<String>,
    page_token: Option<String>,
) -> Result<ListArticulosEtiquetaResult, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .list_articulos(q, page_token.unwrap_or_default())
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
struct SucursalesConfig {
    sucursales: Vec<config::SucursalConfig>,
    default_numalm: Option<String>,
}

#[tauri::command]
fn get_dbf_paths() -> SucursalesConfig {
    let cfg = AppConfig::load().ok();
    SucursalesConfig {
        default_numalm: cfg.as_ref().and_then(|c| c.default_numalm.clone()),
        sucursales: cfg
            .as_ref()
            .map(|c| c.sucursales.clone())
            .unwrap_or_default(),
    }
}

#[tauri::command]
fn save_sucursal_dbf_path(numalm: String, path: String) -> Result<(), String> {
    AppConfig::update_sucursal_dbf_path(&numalm, &path).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_default_numalm(numalm: String) -> Result<(), String> {
    AppConfig::update_default_numalm(&numalm).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_sucursales_map(mapping: Vec<config::SucursalConfig>) -> Result<(), String> {
    AppConfig::update_sucursales(&mapping).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_default_printer() -> Option<String> {
    AppConfig::load().ok().and_then(|c| c.default_printer)
}

#[tauri::command]
fn save_default_printer(printer: String) -> Result<(), String> {
    AppConfig::update_default_printer(&printer).map_err(|e| e.to_string())
}

// ── Estadísticas ───────────────────────────────────────────────

fn numalm_to_branch_letter(numalm: &str, cfg: &Option<AppConfig>) -> Option<char> {
    if let Some(c) = cfg {
        if let Some(entry) = c
            .sucursales
            .iter()
            .find(|s| s.numalm.trim() == numalm.trim())
        {
            return entry.letra.trim().chars().next();
        }
    }
    numalm
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|&n| n >= 1)
        .map(|n| (b'A' + (n - 1) as u8) as char)
}

#[derive(serde::Serialize)]
struct PeriodoStat {
    periodo: String,
    ventas_importe: f64,
    compras_importe: f64,
    ventas_count: u32,
    compras_count: u32,
    facturas_importe: f64,
    facturas_count: u32,
    remisiones_importe: f64,
    remisiones_count: u32,
    notas_importe: f64,
    notas_count: u32,
    credito_importe: f64,
    credito_count: u32,
    abonos_importe: f64,
    abonos_count: u32,
    devoluciones_importe: f64,
    devoluciones_count: u32,
}

#[derive(serde::Serialize)]
struct EstadisticasResult {
    periodos: Vec<PeriodoStat>,
    total_ventas: f64,
    total_compras: f64,
    total_ventas_count: u32,
    total_compras_count: u32,
    total_credito: f64,
    total_credito_count: u32,
    total_abonos: f64,
    total_abonos_count: u32,
    total_devoluciones: f64,
    total_devoluciones_count: u32,
}

#[derive(serde::Serialize)]
struct EstadisticasDosAniosResult {
    actual: EstadisticasResult,
    anterior: EstadisticasResult,
}

fn compute_estadisticas(
    docs: &[models::Documento],
    cxc_records: &[models::Cxc],
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
    branch_filter: Option<char>,
) -> EstadisticasResult {
    use std::collections::HashMap;

    let mut periodos_map: HashMap<String, PeriodoStat> = HashMap::new();

    for doc in docs {
        if doc.deleted_in_dbf {
            continue;
        }

        let tipodoc = doc.tipodoc.trim();
        let es_nota_venta = tipodoc == "N" && doc.formapago.trim() == "1";
        let es_venta =
            (matches!(tipodoc, "R" | "F") && doc.formapago.trim() == "1") || es_nota_venta;
        let es_compra = tipodoc == "C";
        let es_devolucion = matches!(tipodoc, "DN" | "DR");
        if !es_venta && !es_compra && !es_devolucion {
            continue;
        }

        // Excluir facturas que son de diario (FACTDIARIA = false)
        if tipodoc == "F" && doc.factdiaria == Some(true) {
            continue;
        }

        // Notas de venta: excluir solo status 1; demás documentos: solo incluir status 0
        let skip = if es_nota_venta {
            doc.status == 1
        } else {
            doc.status != 0
        };
        if skip {
            continue;
        }

        let fecha = match doc.fechacapt {
            Some(f) => f,
            None => continue,
        };
        if fecha < from || fecha > to {
            continue;
        }

        let periodo = fecha.format("%Y-%m").to_string();
        let entry = periodos_map
            .entry(periodo.clone())
            .or_insert_with(|| PeriodoStat {
                periodo,
                ventas_importe: 0.0,
                compras_importe: 0.0,
                ventas_count: 0,
                compras_count: 0,
                facturas_importe: 0.0,
                facturas_count: 0,
                remisiones_importe: 0.0,
                remisiones_count: 0,
                notas_importe: 0.0,
                notas_count: 0,
                credito_importe: 0.0,
                credito_count: 0,
                abonos_importe: 0.0,
                abonos_count: 0,
                devoluciones_importe: 0.0,
                devoluciones_count: 0,
            });

        let total_doc = doc.importe - doc.descuento + doc.impuesto1 + doc.impuesto2;
        if es_venta {
            entry.ventas_importe += total_doc;
            entry.ventas_count += 1;
            match tipodoc {
                "F" => {
                    entry.facturas_importe += total_doc;
                    entry.facturas_count += 1;
                }
                "R" => {
                    entry.remisiones_importe += total_doc;
                    entry.remisiones_count += 1;
                }
                "N" => {
                    entry.notas_importe += total_doc;
                    entry.notas_count += 1;
                }
                _ => {}
            }
        } else if es_compra {
            entry.compras_importe += total_doc;
            entry.compras_count += 1;
        } else if es_devolucion {
            entry.devoluciones_importe += total_doc;
            entry.devoluciones_count += 1;
        }
    }

    // Acumular abonos CXC
    for cxc in cxc_records {
        if cxc.deleted_in_dbf || cxc.ca.trim() != "1" {
            continue;
        }
        // Filtrar por sucursal: KEYDOCUM formato "F    C15573" → segundo token → primer char
        if let Some(expected) = branch_filter {
            let branch = cxc
                .keydocum
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.chars().next());
            if branch != Some(expected) {
                continue;
            }
        }
        let fecha = match cxc.fecha {
            Some(f) => f,
            None => continue,
        };
        if fecha < from || fecha > to {
            continue;
        }

        let periodo = fecha.format("%Y-%m").to_string();
        let entry = periodos_map
            .entry(periodo.clone())
            .or_insert_with(|| PeriodoStat {
                periodo,
                ventas_importe: 0.0,
                compras_importe: 0.0,
                ventas_count: 0,
                compras_count: 0,
                facturas_importe: 0.0,
                facturas_count: 0,
                remisiones_importe: 0.0,
                remisiones_count: 0,
                notas_importe: 0.0,
                notas_count: 0,
                credito_importe: 0.0,
                credito_count: 0,
                abonos_importe: 0.0,
                abonos_count: 0,
                devoluciones_importe: 0.0,
                devoluciones_count: 0,
            });
        entry.abonos_importe += cxc.importe;
        entry.abonos_count += 1;
    }

    let mut periodos: Vec<PeriodoStat> = periodos_map.into_values().collect();
    periodos.sort_by(|a, b| a.periodo.cmp(&b.periodo));

    EstadisticasResult {
        total_ventas: periodos.iter().map(|p| p.ventas_importe).sum(),
        total_compras: periodos.iter().map(|p| p.compras_importe).sum(),
        total_ventas_count: periodos.iter().map(|p| p.ventas_count).sum(),
        total_compras_count: periodos.iter().map(|p| p.compras_count).sum(),
        total_credito: periodos.iter().map(|p| p.credito_importe).sum(),
        total_credito_count: periodos.iter().map(|p| p.credito_count).sum(),
        total_abonos: periodos.iter().map(|p| p.abonos_importe).sum(),
        total_abonos_count: periodos.iter().map(|p| p.abonos_count).sum(),
        total_devoluciones: periodos.iter().map(|p| p.devoluciones_importe).sum(),
        total_devoluciones_count: periodos.iter().map(|p| p.devoluciones_count).sum(),
        periodos,
    }
}

#[tauri::command]
fn get_estadisticas_docum(
    fecha_from: Option<String>,
    fecha_to: Option<String>,
    numalm: Option<String>,
) -> Result<EstadisticasResult, String> {
    use std::path::Path;

    let cfg = AppConfig::load().ok();
    let numalm_str = numalm.as_deref().unwrap_or("");
    let docum_path = cfg
        .as_ref()
        .and_then(|c| c.docum_path_for(numalm_str))
        .ok_or_else(|| "Archivo Docum.DBF no configurado para este almacén".to_string())?;

    let from_date = fecha_from
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());
    let to_date = fecha_to
        .as_deref()
        .and_then(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok());

    let docs = dbf_reader::read_documentos(Path::new(&docum_path), from_date, to_date)
        .map_err(|e| e.to_string())?;

    let branch_filter = numalm
        .as_deref()
        .and_then(|n| numalm_to_branch_letter(n, &cfg));
    let cxc_min = from_date.and_then(|d| {
        use chrono::Datelike;
        chrono::NaiveDate::from_ymd_opt(d.year() - 1, 1, 1)
    });
    let cxc_records = cfg
        .as_ref()
        .and_then(|c| c.cxc_path_for(numalm_str))
        .and_then(|p| dbf_reader::read_cxc(std::path::Path::new(&p), cxc_min).ok())
        .unwrap_or_default();

    let from = from_date.unwrap_or(chrono::NaiveDate::from_ymd_opt(1900, 1, 1).unwrap());
    let to = to_date.unwrap_or(chrono::NaiveDate::from_ymd_opt(9999, 12, 31).unwrap());

    Ok(compute_estadisticas(
        &docs,
        &cxc_records,
        from,
        to,
        branch_filter,
    ))
}

#[tauri::command]
fn get_estadisticas_dos_anios(
    state: State<'_, DocumState>,
    anio: i32,
    numalm: Option<String>,
) -> Result<EstadisticasDosAniosResult, String> {
    use chrono::NaiveDate;
    use std::path::Path;

    let cfg = AppConfig::load().ok();
    let numalm_str = numalm.as_deref().unwrap_or("");
    let docum_path: String = cfg
        .as_ref()
        .and_then(|c| c.docum_path_for(numalm_str))
        .ok_or_else(|| "Archivo Docum.DBF no configurado para este almacén".to_string())?;

    let from_prev = NaiveDate::from_ymd_opt(anio - 1, 1, 1).unwrap();
    let to_curr = NaiveDate::from_ymd_opt(anio, 12, 31).unwrap();

    // Caché: re-leer solo si el archivo cambió (mtime) o cambió el año
    let docs = {
        let mtime = std::fs::metadata(&docum_path)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

        let mut cache = state.0.lock().unwrap();
        let hit = cache.as_ref().map_or(false, |c| {
            c.path == docum_path && c.mtime == mtime && c.anio == anio
        });

        if hit {
            cache.as_ref().unwrap().docs.clone()
        } else {
            let docs =
                dbf_reader::read_documentos(Path::new(&docum_path), Some(from_prev), Some(to_curr))
                    .map_err(|e| e.to_string())?;
            *cache = Some(DocumCacheEntry {
                path: docum_path.clone(),
                mtime,
                anio,
                docs: docs.clone(),
            });
            docs
        }
    };

    // 1 sola lectura de CXC.DBF
    let branch_filter = numalm
        .as_deref()
        .and_then(|n| numalm_to_branch_letter(n, &cfg));
    let cxc_min = NaiveDate::from_ymd_opt(anio - 2, 1, 1);
    let cxc_records = cfg
        .as_ref()
        .and_then(|c| c.cxc_path_for(numalm_str))
        .and_then(|p| dbf_reader::read_cxc(Path::new(&p), cxc_min).ok())
        .unwrap_or_default();

    let from_curr = NaiveDate::from_ymd_opt(anio, 1, 1).unwrap();
    let to_prev = NaiveDate::from_ymd_opt(anio - 1, 12, 31).unwrap();

    Ok(EstadisticasDosAniosResult {
        actual: compute_estadisticas(&docs, &cxc_records, from_curr, to_curr, branch_filter),
        anterior: compute_estadisticas(&docs, &cxc_records, from_prev, to_prev, branch_filter),
    })
}

#[derive(serde::Serialize)]
struct FraccionesInitData {
    fracciones: Vec<ArticuloFracciones>,
    articulos: Vec<ArticuloSearchResult>,
    etiquetas: Vec<Etiqueta>,
    seguimientos: Vec<SeguimientoFraccionRow>,
}

#[tauri::command]
fn get_fracciones_init_data(numalm: Option<String>) -> Result<FraccionesInitData, String> {
    use std::collections::HashMap;
    use std::path::Path;

    let cfg = AppConfig::load().ok();
    let numalm_str = numalm.as_deref().unwrap_or("");

    let arts_path = cfg
        .as_ref()
        .and_then(|c| c.arts_path_for(numalm_str))
        .ok_or_else(|| "Carpeta DBF no configurada para este almacén".to_string())?;

    let unidades_path = cfg
        .as_ref()
        .and_then(|c| c.unidades_path_for(numalm_str))
        .ok_or_else(|| "Carpeta DBF no configurada para este almacén".to_string())?;

    let articulos_raw =
        dbf_reader::read_articulos(Path::new(&arts_path)).map_err(|e| e.to_string())?;
    let unidades =
        dbf_reader::read_unidades(Path::new(&unidades_path)).map_err(|e| e.to_string())?;

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
                precio1: art.map(|a| a.precio1).unwrap_or(0.0),
                precio2: art.map(|a| a.precio2).unwrap_or(0.0),
                precio3: art.map(|a| a.precio3).unwrap_or(0.0),
                precio4: art.map(|a| a.precio4).unwrap_or(0.0),
                precio5: art.map(|a| a.precio5).unwrap_or(0.0),
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
    let seguimientos = db::get_all_seguimientos().unwrap_or_default();

    Ok(FraccionesInitData {
        fracciones,
        articulos,
        etiquetas,
        seguimientos,
    })
}

#[tauri::command]
fn save_fraccion_pairing(
    numart_origen: String,
    unidad_fraccion: String,
    numart_destino: String,
) -> Result<(), String> {
    db::add_seguimiento(&numart_origen, &unidad_fraccion).map_err(|e| e.to_string())?;
    db::upsert_pairing(&numart_origen, &unidad_fraccion, &numart_destino).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_fraccion_pairing(numart_origen: String, unidad_fraccion: String) -> Result<(), String> {
    db::delete_pairing(&numart_origen, &unidad_fraccion).map_err(|e| e.to_string())
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

// ── Comandos XLSX ──────────────────────────────────────────────

#[tauri::command]
fn export_pairings_template(path: String) -> Result<(), String> {
    xlsx::write_template(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_pairings_xlsx(path: String) -> Result<usize, String> {
    let rows = db::get_all_pairings_vec().map_err(|e| e.to_string())?;
    let count = rows.len();
    xlsx::write_pairings(&path, &rows).map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
fn parse_pairings_xlsx(path: String) -> Result<ParsePairingsResult, String> {
    xlsx::parse_pairings(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_pairings(rows: Vec<PairingRow>, mode: String) -> Result<usize, String> {
    db::import_pairings(&rows, &mode).map_err(|e| e.to_string())
}

// ── Comandos de Seguimientos ───────────────────────────────────

#[tauri::command]
fn add_seguimiento_fraccion(numart_origen: String, unidad_fraccion: String) -> Result<(), String> {
    db::add_seguimiento(&numart_origen, &unidad_fraccion).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_seguimiento_fraccion(
    numart_origen: String,
    unidad_fraccion: String,
) -> Result<(), String> {
    db::delete_seguimiento(&numart_origen, &unidad_fraccion).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_seguimientos_template(path: String) -> Result<(), String> {
    xlsx::write_seguimientos_template(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_seguimientos_xlsx(path: String) -> Result<usize, String> {
    let rows = db::get_all_seguimientos().map_err(|e| e.to_string())?;
    let count = rows.len();
    xlsx::write_seguimientos(&path, &rows).map_err(|e| e.to_string())?;
    Ok(count)
}

#[tauri::command]
fn parse_seguimientos_xlsx(path: String) -> Result<ParseSeguimientosResult, String> {
    xlsx::parse_seguimientos(&path).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_seguimientos(rows: Vec<SeguimientoFraccionRow>, mode: String) -> Result<usize, String> {
    db::import_seguimientos(&rows, &mode).map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_estadisticas_inventario_detalle(
    grpc: State<'_, GrpcState>,
    numalm: Option<String>,
) -> Result<InventarioAnioResult, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .get_estadisticas_inventario_detalle(numalm)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_estadisticas_cxc_mensual(
    grpc: State<'_, GrpcState>,
    numalm: Option<String>,
) -> Result<CxcMensualAnioResult, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .get_estadisticas_cxc_mensual(numalm)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_minv_por_articulo(
    grpc: State<'_, GrpcState>,
    numart: String,
    numalm: Option<String>,
    fecha_from: Option<String>,
    fecha_to: Option<String>,
    page_size: Option<i32>,
    page_token: Option<String>,
) -> Result<ListMinvResult, String> {
    grpc.lock()
        .await
        .as_mut()
        .ok_or_else(|| "Sin configuración".to_string())?
        .list_minv_por_articulo(
            numart,
            numalm,
            fecha_from,
            fecha_to,
            page_size.unwrap_or(50),
            page_token.unwrap_or_default(),
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_inventario_por_mes(
    numalm: String,
    anio: i32,
    mes: u32,
) -> Result<InventarioMesDetalleResult, String> {
    use chrono::NaiveDate;
    use std::collections::HashMap;
    use std::path::Path;

    let cfg = AppConfig::load().map_err(|e| e.to_string())?;
    let arts_path = cfg
        .arts_path_for(&numalm)
        .ok_or_else(|| "Carpeta DBF no configurada para este almacén".to_string())?;
    let minv_path = cfg
        .minv_path_for(&numalm)
        .ok_or_else(|| "Carpeta DBF no configurada para este almacén".to_string())?;

    let articulos = dbf_reader::read_articulos(Path::new(&arts_path))
        .map_err(|e| e.to_string())?;
    let arts_map: HashMap<String, String> = articulos
        .iter()
        .filter(|a| !a.deleted_in_dbf)
        .map(|a| (a.numart.trim().to_string(), a.desc.clone()))
        .collect();

    let mes_start = NaiveDate::from_ymd_opt(anio, mes, 1)
        .ok_or_else(|| format!("Fecha inválida: {}-{}", anio, mes))?;
    let (next_y, next_m) = if mes == 12 { (anio + 1, 1) } else { (anio, mes + 1) };
    let mes_end = NaiveDate::from_ymd_opt(next_y, next_m, 1).unwrap();

    let acum = dbf_reader::aggregate_minv_for_month(
        Path::new(&minv_path),
        mes_start,
        mes_end,
    ).map_err(|e| e.to_string())?;

    let mut result: Vec<ArticuloMovMesStat> = acum
        .into_iter()
        .filter(|(_, (si, e, s))| si.abs() > 1e-6 || *e > 1e-6 || *s > 1e-6)
        .map(|(numart, (si, e, s))| {
            let desc = arts_map.get(&numart).cloned().unwrap_or_default();
            ArticuloMovMesStat {
                numart,
                desc,
                saldo_inicial: si,
                entradas: e,
                salidas: s,
                saldo_final: si + e - s,
            }
        })
        .collect();

    result.sort_by(|a, b| a.numart.cmp(&b.numart));
    Ok(InventarioMesDetalleResult { anio, mes, articulos: result })
}

// ── Entry point ────────────────────────────────────────────────

#[tauri::command]
async fn init_client(grpc: State<'_, GrpcState>) -> Result<bool, String> {
    match AppConfig::load() {
        Ok(config) => {
            match GrpcClient::new(&config.grpc_endpoint, &config.api_key) {
                Ok(client) => {
                    *grpc.lock().await = Some(client);
                    Ok(true)
                }
                Err(_) => Ok(false),
            }
        }
        Err(_) => Ok(false),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("lufal_auxiliar_desktop_lib=debug".parse().unwrap()),
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
            app.manage(DocumState(std::sync::Mutex::new(None)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_almacenes,
            list_documentos,
            get_documento,
            get_proveedor_nombre,
            get_articulos,
            list_articulos_etiqueta,
            buscar_seguimiento,
            get_config_path,
            check_config,
            save_config,
            init_client,
            get_dbf_paths,
            save_sucursal_dbf_path,
            save_default_numalm,
            get_fracciones_init_data,
            save_fraccion_pairing,
            delete_fraccion_pairing,
            get_etiquetas,
            create_etiqueta,
            update_etiqueta,
            delete_etiqueta,
            set_emparejamiento_etiquetas,
            export_pairings_template,
            export_pairings_xlsx,
            parse_pairings_xlsx,
            import_pairings,
            add_seguimiento_fraccion,
            delete_seguimiento_fraccion,
            export_seguimientos_template,
            export_seguimientos_xlsx,
            parse_seguimientos_xlsx,
            import_seguimientos,
            get_estadisticas_docum,
            get_estadisticas_dos_anios,
            get_estadisticas_inventario_detalle,
            get_estadisticas_cxc_mensual,
            save_sucursales_map,
            list_minv_por_articulo,
            get_inventario_por_mes,
            print::list_printers,
            print::print_etiquetas,
            get_default_printer,
            save_default_printer,
        ])
        .run(tauri::generate_context!())
        .expect("Error al iniciar la aplicación Tauri");
}
