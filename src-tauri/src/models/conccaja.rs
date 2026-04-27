pub struct Conccaja {
    pub tipodoc:    String,       // C:5  — PK
    pub desc:       String,       // C:40
    pub conccxc:    String,       // C:2  — referencia blanda a conccxc.conc
    pub tipomov:    String,       // C:1
    pub pidebenef:  Option<bool>, // L:1
    pub grupos:     String,       // C:50
    pub esmovefvo:  Option<bool>, // L:1
    pub piderefer:  Option<bool>, // L:1
    pub pjecom:     f64,          // N:10.5
    pub mostrtot:   Option<bool>, // L:1
    pub usaauto:    Option<bool>, // L:1
    pub gposauto:   String,       // C:50
    pub clavesat:   String,       // C:2
    pub planpagos:  Option<bool>, // L:1
    pub obligarref: Option<bool>, // L:1
    pub provpago:   f64,          // N:1.0
    pub deleted_in_dbf: bool,
}
