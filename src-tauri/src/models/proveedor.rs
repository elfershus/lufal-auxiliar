use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Proveedor {
    pub numprov: String,                      // C:5  - PK
    pub nomprov: String,                      // C:50
    pub calle: String,                        // C:30
    pub numext: String,                       // C:10
    pub colonia: String,                      // C:20
    pub ciudad: String,                       // C:30
    pub estado: String,                       // C:20
    pub cp: String,                           // C:6
    pub telefono: String,                     // C:50
    pub fax: String,                          // C:50
    pub clasif: String,                       // C:5
    pub compano: f64,                         // N:12.2
    pub ultcomp: Option<chrono::NaiveDate>,   // D:8
    pub contacto: String,                     // C:40
    pub rfc: String,                          // C:13
    pub pjedesc: f64,                         // N:6.2
    pub saldo: f64,                           // N:12.2
    pub diascred: f64,                        // N:3.0
    pub numcta: String,                       // C:20
    pub tproviva: f64,                        // N:2.0
    pub diotpais: String,                     // C:2
    pub diotnal: String,                      // C:25
    pub diottaxid: String,                    // C:25
    pub email: String,                        // C:40
    pub email2: String,                       // C:40
    pub email3: String,                       // C:40
    pub contacto2: String,                    // C:40
    pub contacto3: String,                    // C:40
    pub telefono2: String,                    // C:50
    pub telefono3: String,                    // C:50
    pub banco: String,                        // C:30
    // CUENTABAN (C:30) excluded
    // CLAVEBAN (C:30) excluded
    pub idregla: f64,                         // N:5.0
    pub impuesto1: f64,                       // N:6.2
    pub idregimen: String,                    // C:3
    pub deleted_in_dbf: bool,
}
