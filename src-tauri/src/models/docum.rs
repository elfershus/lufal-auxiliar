use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Documento {
    pub tipodoc: String,            // C:2
    pub numdoc: String,             // C:10
    pub numalm: String,             // C:2
    pub fecha: Option<NaiveDate>,   // D:8
    pub numcli: String,             // C:5
    pub numprov: String,            // C:5
    pub formapago: String,          // C:1
    pub pjedesc: f64,               // N:5.2
    pub fechapago: Option<NaiveDate>, // D:8
    pub refer: String,              // C:10
    pub importe: f64,               // N:10.2
    pub descuento: f64,             // N:10.2
    pub impuesto1: f64,             // N:10.2
    pub impuesto2: f64,             // N:10.2
    pub status: i32,                // N:2.0
    pub costo: f64,                 // N:10.2
    pub costo2: f64,                // N:10.2
    pub costopro: f64,              // N:10.2
    pub descuentog: f64,            // N:10.2
    pub hora: String,               // C:8
    pub factdiaria: Option<bool>,   // L:1
    pub fueticket: Option<bool>,    // L:1
    pub retencion1: f64,            // N:10.2
    pub retencion2: f64,            // N:10.2
    pub fechacapt: Option<NaiveDate>, // D:8
    pub corte: String,              // C:7
    pub descuento1: f64,            // N:10.2
    pub descuento2: f64,            // N:10.2
    pub descuento3: f64,            // N:10.2
    pub descuento4: f64,            // N:10.2
    pub fechacanc: Option<NaiveDate>, // D:8
    pub uuid: String,               // C:32
    pub deleted_in_dbf: bool,
}
