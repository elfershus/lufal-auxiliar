use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Almacen {
    pub numalm: String,           // C:2
    pub nomalm: String,           // C:30
    pub niveles: String,          // C:50
    pub ultid: String,            // C:14
    pub ultss: String,            // C:10
    pub salxcapa: Option<bool>,   // L:1
    pub obligacad: Option<bool>,  // L:1
    pub obligalot: Option<bool>,  // L:1
    pub numalmprim: String,       // C:2
    pub calle: String,            // C:50
    pub numext: String,           // C:15
    pub colonia: String,          // C:30
    pub ciudad: String,           // C:50
    pub estado: String,           // C:50
    pub cp: String,               // C:50
    pub deleted_in_dbf: bool,
}
