use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Movimiento {
    pub tipodoc: String,    // C:2
    pub numdoc: String,     // C:10
    pub numpar: String,     // C:3
    pub numart: String,     // C:20
    pub precio: f64,        // N:13.5
    pub costo: f64,         // N:13.5
    pub costo2: f64,        // N:13.5
    pub cant: f64,          // N:10.3
    pub pend: f64,          // N:10.3
    pub pendocant: f64,     // N:10.3
    pub empaque: f64,       // N:10.3
    pub devueltos: f64,     // N:10.3
    pub pjedesc: f64,       // N:5.2
    pub impuesto1: f64,     // N:5.2
    pub impuesto2: f64,     // N:5.2
    pub unidad: String,     // C:5
    pub docant: String,     // C:15
    pub pjedesc2: f64,      // N:6.2
    pub pjedesc3: f64,      // N:6.2
    pub pjedesc4: f64,      // N:6.2
    pub pjedesc1: f64,      // N:6.2
    pub promoid: f64,       // N:5.0
    pub pendcanc: f64,      // N:10.3
    pub deleted_in_dbf: bool,
}
