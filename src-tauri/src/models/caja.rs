use chrono::NaiveDate;

pub struct Caja {
    pub corte:    String,            // C:7  — PK parte 1
    pub numdoc:   String,            // C:5  — PK parte 2
    pub tipodoc:  String,            // C:5  — PK parte 3
    pub tipomov:  String,            // C:1
    pub es:       String,            // C:1
    pub numuser:  String,            // C:5
    pub numalm:   String,            // C:2
    pub fecha:    Option<NaiveDate>, // D:8
    pub hora:     String,            // C:8
    pub numcli:   String,            // C:5
    pub numbenef: String,            // C:6
    pub importe:  f64,               // N:12.2
    pub pago:     f64,               // N:12.2
    pub divisa:   String,            // C:1
    pub tc:       f64,               // N:8.4
    pub cancelado: Option<bool>,     // L:1
    pub keydocum: String,            // C:12
    pub refer:    String,            // C:20
    pub deleted_in_dbf: bool,
}
