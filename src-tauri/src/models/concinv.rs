pub struct Concinv {
    pub conc:     String,  // C:2  — PK (mismo que tipodoc en MINV)
    pub desc:     String,  // C:30
    pub es:       String,  // C:1
    pub cop:      String,  // C:1
    pub formato:  String,  // C:8
    pub niveles:  String,  // C:30
    pub conccanc: String,  // C:2
    pub sigfolio: String,  // C:10
    pub deleted_in_dbf: bool,
}
