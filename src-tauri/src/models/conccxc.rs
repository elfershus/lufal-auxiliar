pub struct Conccxc {
    pub conc:       String,       // C:2  — PK
    pub desc:       String,       // C:20
    pub ca:         String,       // C:1  "1"=cargo, "0"=abono
    pub obligaref:  Option<bool>, // L:1
    pub editar:     Option<bool>, // L:1
    pub reporte:    String,       // C:60
    pub sigfolio:   String,       // C:10
    pub repetido:   Option<bool>, // L:1
    pub clavesat:   String,       // C:2
    pub obligfolio: Option<bool>, // L:1
    pub deleted_in_dbf: bool,
}
