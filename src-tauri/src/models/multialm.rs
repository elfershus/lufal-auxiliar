use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Multialm {
    pub numart:     String,  // C:20
    pub numalm:     String,  // C:2
    pub existencia: f64,     // N:10.3
    pub maximo:     f64,     // N:10.3
    pub minimo:     f64,     // N:10.3
    pub reorden:    f64,     // N:10.3
    pub ubica:      String,  // C:10
    pub deleted_in_dbf: bool,
}
