use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Articulo {
    pub numart: String,              // C:20
    pub desc: String,                // C:60
    pub codigo: String,              // C:20
    pub unidad: String,              // C:5
    pub marca: String,               // C:30
    pub modelo: String,              // C:30
    pub linea: String,               // C:5
    pub familia: String,             // C:5
    pub categoria: String,           // C:5
    pub numdep: String,              // C:11
    pub valdep: String,              // C:9
    pub impuesto1: f64,              // N:5.2
    pub impuesto2: f64,              // N:5.2
    pub numprov: String,             // C:5
    pub precio1: f64,                // N:13.5
    pub precio2: f64,                // N:13.5
    pub precio3: f64,                // N:13.5
    pub precio4: f64,                // N:13.5
    pub precio5: f64,                // N:13.5
    pub ultcosto: f64,               // N:13.5
    pub ultcosto1: f64,              // N:12.4
    pub activo: Option<bool>,        // L:1
    pub excento: Option<bool>,       // L:1
    pub preciopub: f64,              // N:10.2
    pub servicio: Option<bool>,      // L:1
    pub clavesat: String,            // C:8
    pub deleted_in_dbf: bool,
}
