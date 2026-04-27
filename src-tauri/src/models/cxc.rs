use chrono::NaiveDate;

pub struct Cxc {
    pub keycxc:     String,            // C:12 — PK
    pub numcli:     String,            // C:5
    pub conc:       String,            // C:2
    pub numdoc:     String,            // C:10
    pub refer:      String,            // C:12
    pub fecha:      Option<NaiveDate>, // D:8
    pub venc:       Option<NaiveDate>, // D:8
    pub importe:    f64,               // N:12.2
    pub tc:         f64,               // N:8.4
    pub divisa:     String,            // C:1
    pub saldo:      f64,               // N:12.2
    pub ca:         String,            // C:1  "1"=cargo, "0"=abono
    pub entregada:  Option<bool>,      // L:1
    pub obligaref:  Option<bool>,      // L:1
    pub recno:      i64,               // N:8.0
    pub numuser:    String,            // C:5
    pub numalm:     String,            // C:2
    pub keyrefer:   String,            // C:12
    pub keyrefer2:  String,            // C:12
    pub keydocum:   String,            // C:12
    pub keycaja:    String,            // C:12
    pub fechahora:  String,            // C:14 raw "YYYYMMDDHHMMSS"
    pub cvecuenta:  String,            // C:2
    pub idctaorig:  String,            // C:9
    pub idspei:     String,            // C:9
    pub pagodigrel: String,            // C:10
    pub deleted_in_dbf: bool,
}
