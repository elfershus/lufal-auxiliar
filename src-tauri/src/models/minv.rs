use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Minv {
    pub tipodoc:   String,            // C:2  — PK part 1
    pub numdoc:    String,            // C:10 — PK part 2
    pub numpar:    String,            // C:3  — PK part 3
    pub numart:    String,            // C:20
    pub fecha:     Option<NaiveDate>, // D:8
    pub numalm:    String,            // C:2
    pub cant:      f64,               // N:12.3 — signed: positive=entry, negative=exit
    pub disp:      f64,               // N:12.3
    pub precio:    f64,               // N:13.5
    pub costo:     f64,               // N:13.5
    pub costodls:  f64,               // N:13.5
    pub costo2:    f64,               // N:13.5
    pub costopro:  f64,               // N:13.5
    pub numprov:   String,            // C:5
    pub numcli:    String,            // C:5
    pub numuser:   String,            // C:5
    pub caducidad: Option<NaiveDate>, // D:8
    pub lote:      String,            // C:20
    pub refer:     String,            // C:10
    pub cantimpr:  f64,               // N:3.0
    pub idmotivo:  String,            // C:3
    pub fechahora: String,            // C:14 — raw "YYYYMMDDHHMMSS"
    pub deleted_in_dbf: bool,
}
