use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unidad {
    pub numart: String,    // C:20 — PK parte 1
    pub unidad: String,    // C:5  — PK parte 2
    pub equiv1: f64,       // N:10.3 — denominador de la equivalencia
    pub equiv2: f64,       // N:10.3 — numerador de la equivalencia
    pub precio1: f64,      // N:13.5
    pub precio2: f64,      // N:13.5
    pub precio3: f64,      // N:13.5
    pub precio4: f64,      // N:13.5
    pub precio5: f64,      // N:13.5
    pub preciopub: f64,    // N:10.2
    pub deleted_in_dbf: bool,
}
