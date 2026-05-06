use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlmacenRecord {
    pub numalm: String,
    pub nomalm: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentoRecord {
    pub tipodoc: String,
    pub numdoc: String,
    pub numalm: String,
    pub fecha: String,
    pub numprov: String,
    pub refer: String,
    pub importe: f64,
    pub descuento: f64,
    pub impuesto1: f64,
    pub status: i32,
    pub fechacapt: String,
    pub formapago: String,
    pub pjedesc: f64,
    pub fechapago: String,
    pub uuid: String,
    pub costo: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovimientoRecord {
    pub tipodoc: String,
    pub numdoc: String,
    pub numpar: String,
    pub numart: String,
    pub precio: f64,
    pub cant: f64,
    pub pend: f64,
    pub pjedesc: f64,
    pub impuesto1: f64,
    pub impuesto2: f64,
    pub unidad: String,
    pub docant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDocumentosParams {
    pub tipodoc: Option<String>,
    pub numalm: Option<String>,
    pub fecha_from: Option<String>,
    pub fecha_to: Option<String>,
    pub fechacapt_from: Option<String>,
    pub fechacapt_to: Option<String>,
    pub numdoc: Option<String>,
    pub status: Option<i32>,
    pub page_size: Option<i32>,
    pub page_token: Option<String>,
    pub order_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListDocumentosResult {
    pub documentos: Vec<DocumentoRecord>,
    pub next_page_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetDocumentoResult {
    pub documento: DocumentoRecord,
    pub movimientos: Vec<MovimientoRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompraMatchItem {
    pub tipodoc: String,
    pub numdoc: String,
    pub fecha: String,
    pub numprov: String,
    pub importe: f64,
    pub status: i32,
    pub arts_matched: i32,
    pub total_arts: i32,
    pub coverage_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemisionMatchItem {
    pub tipodoc: String,
    pub numdoc: String,
    pub fecha: String,
    pub numcli: String,
    pub importe: f64,
    pub status: i32,
    pub arts_matched: i32,
    pub total_arts: i32,
    pub coverage_pct: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeguimientoResult {
    pub compras: Vec<CompraMatchItem>,
    pub remisiones: Vec<RemisionMatchItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticuloInfo {
    pub numart: String,
    pub desc: String,
    pub unidad: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticuloPareado {
    pub numart: String,
    pub desc: String,
    pub unidad: String,
    pub precio1: f64,
    pub precio2: f64,
    pub precio3: f64,
    pub precio4: f64,
    pub precio5: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticuloSearchResult {
    pub numart: String,
    pub desc: String,
    pub unidad: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Etiqueta {
    pub id: i64,
    pub nombre: String,
    pub color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraccionRecord {
    pub unidad: String,
    pub equiv1: f64,
    pub equiv2: f64,
    pub precio1: f64,
    pub precio2: f64,
    pub precio3: f64,
    pub precio4: f64,
    pub precio5: f64,
    pub pareado: Option<ArticuloPareado>,
    pub etiquetas: Vec<Etiqueta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticuloFracciones {
    pub numart: String,
    pub desc: String,
    pub unidad_base: String,
    pub precio1: f64,
    pub precio2: f64,
    pub precio3: f64,
    pub precio4: f64,
    pub precio5: f64,
    pub fracciones: Vec<FraccionRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRow {
    pub numart_origen:   String,
    pub unidad_fraccion: String,
    pub numart_destino:  String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingRowPreview {
    pub row_index:       usize,
    pub numart_origen:   String,
    pub unidad_fraccion: String,
    pub numart_destino:  String,
    pub errors:          Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsePairingsResult {
    pub rows:        Vec<PairingRowPreview>,
    pub total_rows:  usize,
    pub valid_count: usize,
    pub error_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeguimientoFraccionRow {
    pub numart_origen:   String,
    pub unidad_fraccion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeguimientoFraccionPreview {
    pub row_index:       usize,
    pub numart_origen:   String,
    pub unidad_fraccion: String,
    pub errors:          Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseSeguimientosResult {
    pub rows:        Vec<SeguimientoFraccionPreview>,
    pub total_rows:  usize,
    pub valid_count: usize,
    pub error_count: usize,
}
