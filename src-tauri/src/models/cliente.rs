use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cliente {
    pub numcli: String,                       // C:5  - PK
    pub nomcli: String,                       // C:150
    pub calle: String,                        // C:60
    pub numext: String,                       // C:10
    pub colonia: String,                      // C:20
    pub ciudad: String,                       // C:30
    pub estado: String,                       // C:20
    pub cp: String,                           // C:6
    pub telefono: String,                     // C:30
    pub fax: String,                          // C:20
    pub clasif: String,                       // C:5
    pub ventano: f64,                         // N:12.2
    pub ultvent: Option<chrono::NaiveDate>,   // D:8
    pub atvent: String,                       // C:40
    pub atcobr: String,                       // C:40
    pub email1: String,                       // C:40
    pub email2: String,                       // C:40
    pub rfc: String,                          // C:13
    pub limcred: f64,                         // N:12.2
    pub saldo: f64,                           // N:12.2
    pub pjedesc: f64,                         // N:5.2
    pub diascred: f64,                        // N:3.0
    pub precioutil: String,                   // C:1
    pub recepfac: String,                     // C:30
    pub pagofac: String,                      // C:30
    pub numcta: String,                       // C:20
    pub uid: f64,                             // N:10.0
    pub numvend: String,                      // C:5
    pub obligareq: Option<bool>,              // L:1
    pub suspendido: Option<bool>,             // L:1
    pub impuesto1: f64,                       // N:6.2
    pub retencion1: f64,                      // N:10.4
    pub retencion2: f64,                      // N:6.2
    pub permitecod: Option<bool>,             // L:1
    pub llavecred: Option<bool>,              // L:1
    pub pais: String,                         // C:15
    pub clavecli: String,                     // C:20
    pub curp: String,                         // C:20
    pub nomcomer: String,                     // C:40
    pub statusweb: f64,                       // N:1.0
    // CLAVEWEB (C:32) excluded
    pub numzona: String,                      // C:5
    pub metodousar: String,                   // C:2
    pub numint: String,                       // C:10
    pub usocfdi: String,                      // C:3
    pub formapago: String,                    // C:2
    pub condpago: String,                     // C:30
    pub emailtw: String,                      // C:50
    pub numidtrib: String,                    // C:40
    pub idregimen: String,                    // C:3
    pub deleted_in_dbf: bool,
}
