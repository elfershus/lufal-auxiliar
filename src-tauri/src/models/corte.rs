use chrono::NaiveDate;

pub struct Corte {
    pub corte:      String,            // C:7  — PK
    pub numalm:     String,            // C:2
    pub numuser:    String,            // C:5
    pub fecha:      Option<NaiveDate>, // D:8
    pub hora:       String,            // C:8
    pub numest:     String,            // C:2
    pub numuserfin: String,            // C:5
    pub fechafin:   Option<NaiveDate>, // D:8
    pub horafin:    String,            // C:8
    pub numusercc:  String,            // C:5
    pub fechacc:    Option<NaiveDate>, // D:8
    pub horacc:     String,            // C:8
    pub numop:      f64,               // N:5.0
    pub entpagos:   f64,               // N:12.2
    pub salcorte:   f64,               // N:12.2
    pub entotrmov:  f64,               // N:12.2
    pub salotrmov:  f64,               // N:12.2
    pub efectivo:   f64,               // N:12.2
    pub tc:         f64,               // N:8.4
    pub cantimpr:   f64,               // N:2.0
    pub modificado: Option<bool>,      // L:1
    pub deleted_in_dbf: bool,
}
