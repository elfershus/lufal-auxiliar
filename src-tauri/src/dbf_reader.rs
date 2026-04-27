use anyhow::{bail, Context, Result};
use chrono::NaiveDate;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::debug;

use crate::models::{Almacen, Articulo, Caja, Cliente, Concinv, Conccxc, Conccaja, Corte, Cxc, Documento, Minv, Movimiento, Multialm, Proveedor, Unidad};

// ── CP850 lookup table (0x80–0xFF) ────────────────────────────────────────────

#[rustfmt::skip]
static CP850_HIGH: [char; 128] = [
    // 0x80
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    // 0x90
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '×', 'ƒ',
    // 0xA0
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '®', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0
    '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©', '╣', '║', '╗', '╝', '¢', '¥', '┐',
    // 0xC0
    '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦', '╠', '═', '╬', '¤',
    // 0xD0
    'ð', 'Ð', 'Ê', 'Ë', 'È', 'ı', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì', '▀',
    // 0xE0
    'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', 'þ', 'Þ', 'Ú', 'Û', 'Ù', 'ý', 'Ý', '¯', '´',
    // 0xF0
    '\u{00AD}', '±', '‗', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', '\u{00A0}',
];

pub fn decode_cp850(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if b < 0x80 {
                b as char
            } else {
                CP850_HIGH[(b - 0x80) as usize]
            }
        })
        .collect()
}

// ── DBF header / field structures ─────────────────────────────────────────────

#[derive(Debug)]
struct FieldDesc {
    name: String,
    field_type: char,
    length: usize,
    // decimal is stored in DBF but we use it only for documentation
}

#[derive(Debug)]
struct DbfHeader {
    num_records: u32,
    header_size: u16,
    record_size: u16,
    fields: Vec<FieldDesc>,
}

fn parse_header(data: &[u8]) -> Result<DbfHeader> {
    if data.len() < 32 {
        bail!("DBF file too short to contain a header");
    }
    let num_records = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
    let header_size = u16::from_le_bytes([data[8], data[9]]);
    let record_size = u16::from_le_bytes([data[10], data[11]]);

    let mut fields = Vec::new();
    let mut offset = 32usize;

    loop {
        if offset >= data.len() {
            bail!("DBF header: unexpected end of file while reading field descriptors");
        }
        if data[offset] == 0x0D {
            break;
        }
        if offset + 32 > data.len() {
            bail!("DBF header: truncated field descriptor at offset {offset}");
        }

        // Field name: 11 bytes, null-terminated
        let name_bytes = &data[offset..offset + 11];
        let name_end = name_bytes.iter().position(|&b| b == 0).unwrap_or(11);
        let name = std::str::from_utf8(&name_bytes[..name_end])
            .unwrap_or("")
            .to_uppercase();

        let field_type = data[offset + 11] as char;
        let length = data[offset + 16] as usize;

        fields.push(FieldDesc { name, field_type, length });
        offset += 32;
    }

    Ok(DbfHeader { num_records, header_size, record_size, fields })
}

// ── Field offset map: name → (byte_offset_within_record, length, type) ────────

fn build_field_map(fields: &[FieldDesc]) -> HashMap<String, (usize, usize, char)> {
    let mut map = HashMap::new();
    let mut offset = 1usize; // first byte is the deletion flag
    for f in fields {
        map.insert(f.name.clone(), (offset, f.length, f.field_type));
        offset += f.length;
    }
    map
}

// ── Low-level field parsers ────────────────────────────────────────────────────

fn get_field<'a>(
    record: &'a [u8],
    map: &HashMap<String, (usize, usize, char)>,
    name: &str,
) -> &'a [u8] {
    match map.get(name) {
        Some(&(offset, length, _)) => {
            let end = (offset + length).min(record.len());
            if offset >= record.len() {
                &[]
            } else {
                &record[offset..end]
            }
        }
        None => &[],
    }
}

fn parse_char(raw: &[u8]) -> String {
    let s = decode_cp850(raw);
    // Strip null padding first, then trim all Unicode whitespace (including U+00A0)
    // so the result is consistent with the .trim() calls used when building error keys.
    s.trim_matches('\0').trim().to_string()
}

fn parse_numeric(raw: &[u8]) -> f64 {
    std::str::from_utf8(raw)
        .unwrap_or("")
        .trim()
        .parse()
        .unwrap_or(0.0)
}

fn parse_int(raw: &[u8]) -> i32 {
    std::str::from_utf8(raw)
        .unwrap_or("")
        .trim()
        .parse()
        .unwrap_or(0)
}

fn parse_date(raw: &[u8]) -> Option<NaiveDate> {
    let s = std::str::from_utf8(raw).unwrap_or("").trim();
    if s.is_empty() || s == "00000000" {
        return None;
    }
    NaiveDate::parse_from_str(s, "%Y%m%d").ok()
}

fn parse_logical(raw: &[u8]) -> Option<bool> {
    match raw.first() {
        Some(b'T') | Some(b't') | Some(b'Y') | Some(b'y') => Some(true),
        Some(b'F') | Some(b'f') => Some(false),
        // 'N'/'n' and ' ' are both "not initialized" in VFP → treat as null
        _ => None,
    }
}

// ── Public readers ─────────────────────────────────────────────────────────────

pub fn read_documentos(path: &Path) -> Result<Vec<Documento>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo DOCUM.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! d { ($f:expr) => { parse_date(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }
        macro_rules! ni { ($f:expr) => { parse_int(get_field(rec, &map, $f)) }; }

        result.push(Documento {
            tipodoc: c!("TIPODOC"),
            numdoc: c!("NUMDOC"),
            numalm: c!("NUMALM"),
            fecha: d!("FECHA"),
            numcli: c!("NUMCLI"),
            numprov: c!("NUMPROV"),
            formapago: c!("FORMAPAGO"),
            pjedesc: n!("PJEDESC"),
            fechapago: d!("FECHAPAGO"),
            refer: c!("REFER"),
            importe: n!("IMPORTE"),
            descuento: n!("DESCUENTO"),
            impuesto1: n!("IMPUESTO1"),
            impuesto2: n!("IMPUESTO2"),
            status: ni!("STATUS"),
            costo: n!("COSTO"),
            costo2: n!("COSTO2"),
            costopro: n!("COSTOPRO"),
            descuentog: n!("DESCUENTOG"),
            hora: c!("HORA"),
            factdiaria: l!("FACTDIARIA"),
            fueticket: l!("FUETICKET"),
            retencion1: n!("RETENCION1"),
            retencion2: n!("RETENCION2"),
            fechacapt: d!("FECHACAPT"),
            corte: c!("CORTE"),
            descuento1: n!("DESCUENTO1"),
            descuento2: n!("DESCUENTO2"),
            descuento3: n!("DESCUENTO3"),
            descuento4: n!("DESCUENTO4"),
            fechacanc: d!("FECHACANC"),
            uuid: c!("UUID"),
            deleted_in_dbf,
        });
        {
            let d = result.last().unwrap();
            debug!(row = i, tipodoc = %d.tipodoc, numdoc = %d.numdoc,
                   status = d.status, fecha = ?d.fecha, importe = d.importe,
                   numcli = %d.numcli, deleted = d.deleted_in_dbf,
                   "dbf_reader: documento leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: DOCUM.DBF completo");
    Ok(result)
}

pub fn read_movimientos(path: &Path) -> Result<Vec<Movimiento>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo MOVIM.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }

        result.push(Movimiento {
            tipodoc: c!("TIPODOC"),
            numdoc: c!("NUMDOC"),
            numpar: c!("NUMPAR"),
            numart: c!("NUMART"),
            precio: n!("PRECIO"),
            costo: n!("COSTO"),
            costo2: n!("COSTO2"),
            cant: n!("CANT"),
            pend: n!("PEND"),
            pendocant: n!("PENDOCANT"),
            empaque: n!("EMPAQUE"),
            devueltos: n!("DEVUELTOS"),
            pjedesc: n!("PJEDESC"),
            impuesto1: n!("IMPUESTO1"),
            impuesto2: n!("IMPUESTO2"),
            unidad: c!("UNIDAD"),
            docant: c!("DOCANT"),
            pjedesc2: n!("PJEDESC2"),
            pjedesc3: n!("PJEDESC3"),
            pjedesc4: n!("PJEDESC4"),
            pjedesc1: n!("PJEDESC1"),
            promoid: n!("PROMOID"),
            pendcanc: n!("PENDCANC"),
            deleted_in_dbf,
        });
        {
            let m = result.last().unwrap();
            debug!(row = i, tipodoc = %m.tipodoc, numdoc = %m.numdoc,
                   numpar = %m.numpar, numart = %m.numart, cant = m.cant,
                   precio = m.precio, deleted = m.deleted_in_dbf,
                   "dbf_reader: movimiento leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: MOVIM.DBF completo");
    Ok(result)
}

pub fn read_articulos(path: &Path) -> Result<Vec<Articulo>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo ARTS.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        result.push(Articulo {
            numart: c!("NUMART"),
            desc: c!("DESC"),
            codigo: c!("CODIGO"),
            unidad: c!("UNIDAD"),
            marca: c!("MARCA"),
            modelo: c!("MODELO"),
            linea: c!("LINEA"),
            familia: c!("FAMILIA"),
            categoria: c!("CATEGORIA"),
            numdep: c!("NUMDEP"),
            valdep: c!("VALDEP"),
            impuesto1: n!("IMPUESTO1"),
            impuesto2: n!("IMPUESTO2"),
            numprov: c!("NUMPROV"),
            precio1: n!("PRECIO1"),
            precio2: n!("PRECIO2"),
            precio3: n!("PRECIO3"),
            precio4: n!("PRECIO4"),
            precio5: n!("PRECIO5"),
            ultcosto: n!("ULTCOSTO"),
            ultcosto1: n!("ULTCOSTO1"),
            activo: l!("ACTIVO"),
            excento: l!("EXCENTO"),
            preciopub: n!("PRECIOPUB"),
            servicio: l!("SERVICIO"),
            clavesat: c!("CLAVESAT"),
            deleted_in_dbf,
        });
        {
            let a = result.last().unwrap();
            debug!(row = i, numart = %a.numart, desc = %a.desc,
                   precio1 = a.precio1, activo = ?a.activo,
                   deleted = a.deleted_in_dbf,
                   "dbf_reader: artículo leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: ARTS.DBF completo");
    Ok(result)
}

pub fn read_proveedores(path: &Path) -> Result<Vec<Proveedor>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo PROVEDOR.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! d { ($f:expr) => { parse_date(get_field(rec, &map, $f)) }; }

        result.push(Proveedor {
            numprov:   c!("NUMPROV"),
            nomprov:   c!("NOMPROV"),
            calle:     c!("CALLE"),
            numext:    c!("NUMEXT"),
            colonia:   c!("COLONIA"),
            ciudad:    c!("CIUDAD"),
            estado:    c!("ESTADO"),
            cp:        c!("CP"),
            telefono:  c!("TELEFONO"),
            fax:       c!("FAX"),
            clasif:    c!("CLASIF"),
            compano:   n!("COMPANO"),
            ultcomp:   d!("ULTCOMP"),
            contacto:  c!("CONTACTO"),
            rfc:       c!("RFC"),
            pjedesc:   n!("PJEDESC"),
            saldo:     n!("SALDO"),
            diascred:  n!("DIASCRED"),
            numcta:    c!("NUMCTA"),
            tproviva:  n!("TPROVIVA"),
            diotpais:  c!("DIOTPAIS"),
            diotnal:   c!("DIOTNAL"),
            diottaxid: c!("DIOTTAXID"),
            email:     c!("EMAIL"),
            email2:    c!("EMAIL2"),
            email3:    c!("EMAIL3"),
            contacto2: c!("CONTACTO2"),
            contacto3: c!("CONTACTO3"),
            telefono2: c!("TELEFONO2"),
            telefono3: c!("TELEFONO3"),
            banco:     c!("BANCO"),
            // CUENTABAN and CLAVEBAN excluded
            idregla:   n!("IDREGLA"),
            impuesto1: n!("IMPUESTO1"),
            idregimen: c!("IDREGIMEN"),
            deleted_in_dbf,
        });
        {
            let p = result.last().unwrap();
            debug!(row = i, numprov = %p.numprov, nomprov = %p.nomprov,
                   rfc = %p.rfc, deleted = p.deleted_in_dbf,
                   "dbf_reader: proveedor leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: PROVEDOR.DBF completo");
    Ok(result)
}

pub fn read_clientes(path: &Path) -> Result<Vec<Cliente>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo CLIENTES.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! d { ($f:expr) => { parse_date(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        result.push(Cliente {
            numcli:     c!("NUMCLI"),
            nomcli:     c!("NOMCLI"),
            calle:      c!("CALLE"),
            numext:     c!("NUMEXT"),
            colonia:    c!("COLONIA"),
            ciudad:     c!("CIUDAD"),
            estado:     c!("ESTADO"),
            cp:         c!("CP"),
            telefono:   c!("TELEFONO"),
            fax:        c!("FAX"),
            clasif:     c!("CLASIF"),
            ventano:    n!("VENTANO"),
            ultvent:    d!("ULTVENT"),
            atvent:     c!("ATVENT"),
            atcobr:     c!("ATCOBR"),
            email1:     c!("EMAIL1"),
            email2:     c!("EMAIL2"),
            rfc:        c!("RFC"),
            limcred:    n!("LIMCRED"),
            saldo:      n!("SALDO"),
            pjedesc:    n!("PJEDESC"),
            diascred:   n!("DIASCRED"),
            precioutil: c!("PRECIOUTIL"),
            recepfac:   c!("RECEPFAC"),
            pagofac:    c!("PAGOFAC"),
            numcta:     c!("NUMCTA"),
            uid:        n!("UID"),
            numvend:    c!("NUMVEND"),
            obligareq:  l!("OBLIGAREQ"),
            suspendido: l!("SUSPENDIDO"),
            impuesto1:  n!("IMPUESTO1"),
            retencion1: n!("RETENCION1"),
            retencion2: n!("RETENCION2"),
            permitecod: l!("PERMITECOD"),
            llavecred:  l!("LLAVECRED"),
            pais:       c!("PAIS"),
            clavecli:   c!("CLAVECLI"),
            curp:       c!("CURP"),
            nomcomer:   c!("NOMCOMER"),
            statusweb:  n!("STATUSWEB"),
            // CLAVEWEB excluded
            numzona:    c!("NUMZONA"),
            metodousar: c!("METODOUSAR"),
            numint:     c!("NUMINT"),
            usocfdi:    c!("USOCFDI"),
            formapago:  c!("FORMAPAGO"),
            condpago:   c!("CONDPAGO"),
            emailtw:    c!("EMAILTW"),
            numidtrib:  c!("NUMIDTRIB"),
            idregimen:  c!("IDREGIMEN"),
            deleted_in_dbf,
        });
        {
            let cl = result.last().unwrap();
            debug!(row = i, numcli = %cl.numcli, nomcli = %cl.nomcli,
                   rfc = %cl.rfc, deleted = cl.deleted_in_dbf,
                   "dbf_reader: cliente leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: CLIENTES.DBF completo");
    Ok(result)
}

pub fn read_almacenes(path: &Path) -> Result<Vec<Almacen>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo ALMACEN.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        result.push(Almacen {
            numalm:    c!("NUMALM"),
            nomalm:    c!("NOMALM"),
            niveles:   c!("NIVELES"),
            ultid:     c!("ULTID"),
            ultss:     c!("ULTSS"),
            salxcapa:  l!("SALXCAPA"),
            obligacad: l!("OBLIGACAD"),
            obligalot: l!("OBLIGALOT"),
            numalmprim: c!("NUMALMPRIM"),
            calle:     c!("CALLE"),
            numext:    c!("NUMEXT"),
            colonia:   c!("COLONIA"),
            ciudad:    c!("CIUDAD"),
            estado:    c!("ESTADO"),
            cp:        c!("CP"),
            deleted_in_dbf,
        });
        {
            let a = result.last().unwrap();
            debug!(row = i, numalm = %a.numalm, nomalm = %a.nomalm,
                   ciudad = %a.ciudad, deleted = a.deleted_in_dbf,
                   "dbf_reader: almacén leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: ALMACEN.DBF completo");
    Ok(result)
}

pub fn read_multialm(path: &Path) -> Result<Vec<Multialm>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo MULTIALM.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }

        result.push(Multialm {
            numart:     c!("NUMART"),
            numalm:     c!("NUMALM"),
            existencia: n!("EXISTENCIA"),
            maximo:     n!("MAXIMO"),
            minimo:     n!("MINIMO"),
            reorden:    n!("REORDEN"),
            ubica:      c!("UBICA"),
            deleted_in_dbf,
        });
        {
            let m = result.last().unwrap();
            debug!(row = i, numart = %m.numart, numalm = %m.numalm,
                   existencia = m.existencia, deleted = m.deleted_in_dbf,
                   "dbf_reader: multialm leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: MULTIALM.DBF completo");
    Ok(result)
}

pub fn read_minv(path: &Path) -> Result<Vec<Minv>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo MINV.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! d { ($f:expr) => { parse_date(get_field(rec, &map, $f)) }; }

        result.push(Minv {
            tipodoc:   c!("TIPODOC"),
            numdoc:    c!("NUMDOC"),
            numpar:    c!("NUMPAR"),
            numart:    c!("NUMART"),
            fecha:     d!("FECHA"),
            numalm:    c!("NUMALM"),
            cant:      n!("CANT"),
            disp:      n!("DISP"),
            precio:    n!("PRECIO"),
            costo:     n!("COSTO"),
            costodls:  n!("COSTODLS"),
            costo2:    n!("COSTO2"),
            costopro:  n!("COSTOPRO"),
            numprov:   c!("NUMPROV"),
            numcli:    c!("NUMCLI"),
            numuser:   c!("NUMUSER"),
            caducidad: d!("CADUCIDAD"),
            lote:      c!("LOTE"),
            refer:     c!("REFER"),
            cantimpr:  n!("CANTIMPR"),
            idmotivo:  c!("IDMOTIVO"),
            fechahora: c!("FECHAHORA"),
            deleted_in_dbf,
        });
        {
            let m = result.last().unwrap();
            debug!(row = i, tipodoc = %m.tipodoc, numdoc = %m.numdoc,
                   numpar = %m.numpar, numart = %m.numart, cant = m.cant,
                   numalm = %m.numalm, deleted = m.deleted_in_dbf,
                   "dbf_reader: minv leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: MINV.DBF completo");
    Ok(result)
}

pub fn read_concinv(path: &Path) -> Result<Vec<Concinv>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo CONCINV.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }

        result.push(Concinv {
            conc:     c!("CONC"),
            desc:     c!("DESC"),
            es:       c!("ES"),
            cop:      c!("COP"),
            formato:  c!("FORMATO"),
            niveles:  c!("NIVELES"),
            conccanc: c!("CONCCANC"),
            sigfolio: c!("SIGFOLIO"),
            deleted_in_dbf,
        });
        {
            let r = result.last().unwrap();
            debug!(row = i, conc = %r.conc, desc = %r.desc,
                   deleted = r.deleted_in_dbf, "dbf_reader: concinv leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: CONCINV.DBF completo");
    Ok(result)
}

pub fn read_conccxc(path: &Path) -> Result<Vec<Conccxc>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo ConCxc.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        result.push(Conccxc {
            conc:       c!("CONC"),
            desc:       c!("DESC"),
            ca:         c!("CA"),
            obligaref:  l!("OBLIGAREF"),
            editar:     l!("EDITAR"),
            reporte:    c!("REPORTE"),
            sigfolio:   c!("SIGFOLIO"),
            repetido:   l!("REPETIDO"),
            clavesat:   c!("CLAVESAT"),
            obligfolio: l!("OBLIGFOLIO"),
            deleted_in_dbf,
        });
        {
            let r = result.last().unwrap();
            debug!(row = i, conc = %r.conc, desc = %r.desc,
                   deleted = r.deleted_in_dbf, "dbf_reader: conccxc leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: ConCxc.DBF completo");
    Ok(result)
}

pub fn read_cxc(path: &Path) -> Result<Vec<Cxc>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo Cxc.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! d { ($f:expr) => { parse_date(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        result.push(Cxc {
            keycxc:     c!("KEYCXC"),
            numcli:     c!("NUMCLI"),
            conc:       c!("CONC"),
            numdoc:     c!("NUMDOC"),
            refer:      c!("REFER"),
            fecha:      d!("FECHA"),
            venc:       d!("VENC"),
            importe:    n!("IMPORTE"),
            tc:         n!("TC"),
            divisa:     c!("DIVISA"),
            saldo:      n!("SALDO"),
            ca:         c!("CA"),
            entregada:  l!("ENTREGADA"),
            obligaref:  l!("OBLIGAREF"),
            recno:      n!("RECNO") as i64,
            numuser:    c!("NUMUSER"),
            numalm:     c!("NUMALM"),
            keyrefer:   c!("KEYREFER"),
            keyrefer2:  c!("KEYREFER2"),
            keydocum:   c!("KEYDOCUM"),
            keycaja:    c!("KEYCAJA"),
            fechahora:  c!("FECHAHORA"),
            cvecuenta:  c!("CVECUENTA"),
            idctaorig:  c!("IDCTAORIG"),
            idspei:     c!("IDSPEI"),
            pagodigrel: c!("PAGODIGREL"),
            deleted_in_dbf,
        });
        {
            let r = result.last().unwrap();
            debug!(row = i, keycxc = %r.keycxc, numcli = %r.numcli,
                   saldo = r.saldo, deleted = r.deleted_in_dbf, "dbf_reader: cxc leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: Cxc.DBF completo");
    Ok(result)
}

pub fn read_unidades(path: &Path) -> Result<Vec<Unidad>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo Unidades.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }

        result.push(Unidad {
            numart:    c!("NUMART"),
            unidad:    c!("UNIDAD"),
            equiv1:    n!("EQUIV1"),
            equiv2:    n!("EQUIV2"),
            precio1:   n!("PRECIO1"),
            precio2:   n!("PRECIO2"),
            precio3:   n!("PRECIO3"),
            precio4:   n!("PRECIO4"),
            precio5:   n!("PRECIO5"),
            preciopub: n!("PRECIOPUB"),
            deleted_in_dbf,
        });
        {
            let u = result.last().unwrap();
            debug!(row = i, numart = %u.numart, unidad = %u.unidad,
                   equiv1 = u.equiv1, equiv2 = u.equiv2, deleted = u.deleted_in_dbf,
                   "dbf_reader: unidad leída");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: Unidades.DBF completo");
    Ok(result)
}

pub fn read_conccaja(path: &Path) -> Result<Vec<Conccaja>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo ConcCaja.DBF");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        result.push(Conccaja {
            tipodoc:    c!("TIPODOC"),
            desc:       c!("DESC"),
            conccxc:    c!("CONCCXC"),
            tipomov:    c!("TIPOMOV"),
            pidebenef:  l!("PIDEBENEF"),
            grupos:     c!("GRUPOS"),
            esmovefvo:  l!("ESMOVEFVO"),
            piderefer:  l!("PIDEREFER"),
            pjecom:     n!("PJECOM"),
            mostrtot:   l!("MOSTRTOT"),
            usaauto:    l!("USAAUTO"),
            gposauto:   c!("GPOSAUTO"),
            clavesat:   c!("CLAVESAT"),
            planpagos:  l!("PLANPAGOS"),
            obligarref: l!("OBLIGARREF"),
            provpago:   n!("PROVPAGO"),
            deleted_in_dbf,
        });
        {
            let r = result.last().unwrap();
            debug!(row = i, tipodoc = %r.tipodoc, desc = %r.desc,
                   deleted = r.deleted_in_dbf, "dbf_reader: conccaja leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: ConcCaja.DBF completo");
    Ok(result)
}

pub fn read_cortes(path: &Path) -> Result<Vec<Corte>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo cortes2.dbf");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! d { ($f:expr) => { parse_date(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        result.push(Corte {
            corte:      c!("CORTE"),
            numalm:     c!("NUMALM"),
            numuser:    c!("NUMUSER"),
            fecha:      d!("FECHA"),
            hora:       c!("HORA"),
            numest:     c!("NUMEST"),
            numuserfin: c!("NUMUSERFIN"),
            fechafin:   d!("FECHAFIN"),
            horafin:    c!("HORAFIN"),
            numusercc:  c!("NUMUSERCC"),
            fechacc:    d!("FECHACC"),
            horacc:     c!("HORACC"),
            numop:      n!("NUMOP"),
            entpagos:   n!("ENTPAGOS"),
            salcorte:   n!("SALCORTE"),
            entotrmov:  n!("ENTOTRMOV"),
            salotrmov:  n!("SALOTRMOV"),
            efectivo:   n!("EFECTIVO"),
            tc:         n!("TC"),
            cantimpr:   n!("CANTIMPR"),
            modificado: l!("MODIFICADO"),
            deleted_in_dbf,
        });
        {
            let r = result.last().unwrap();
            debug!(row = i, corte = %r.corte, numalm = %r.numalm,
                   deleted = r.deleted_in_dbf, "dbf_reader: corte leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: cortes2.dbf completo");
    Ok(result)
}

pub fn read_caja(path: &Path) -> Result<Vec<Caja>> {
    tracing::info!(path = %path.display(), "dbf_reader: abriendo caja.dbf");
    let data = fs::read(path)
        .with_context(|| format!("Cannot read DBF file: {}", path.display()))?;

    let header = parse_header(&data)?;
    let map = build_field_map(&header.fields);
    let rec_size = header.record_size as usize;
    let data_start = header.header_size as usize;

    let mut result = Vec::with_capacity(header.num_records as usize);

    for i in 0..header.num_records as usize {
        let start = data_start + i * rec_size;
        if start + rec_size > data.len() {
            break;
        }
        let rec = &data[start..start + rec_size];
        let deleted_in_dbf = rec[0] == b'*';

        macro_rules! c { ($f:expr) => { parse_char(get_field(rec, &map, $f)) }; }
        macro_rules! n { ($f:expr) => { parse_numeric(get_field(rec, &map, $f)) }; }
        macro_rules! d { ($f:expr) => { parse_date(get_field(rec, &map, $f)) }; }
        macro_rules! l { ($f:expr) => { parse_logical(get_field(rec, &map, $f)) }; }

        // DESGLOSE (M:10 memo field) is intentionally skipped
        result.push(Caja {
            corte:    c!("CORTE"),
            numdoc:   c!("NUMDOC"),
            tipodoc:  c!("TIPODOC"),
            tipomov:  c!("TIPOMOV"),
            es:       c!("ES"),
            numuser:  c!("NUMUSER"),
            numalm:   c!("NUMALM"),
            fecha:    d!("FECHA"),
            hora:     c!("HORA"),
            numcli:   c!("NUMCLI"),
            numbenef: c!("NUMBENEF"),
            importe:  n!("IMPORTE"),
            pago:     n!("PAGO"),
            divisa:   c!("DIVISA"),
            tc:       n!("TC"),
            cancelado: l!("CANCELADO"),
            keydocum: c!("KEYDOCUM"),
            refer:    c!("REFER"),
            deleted_in_dbf,
        });
        {
            let r = result.last().unwrap();
            debug!(row = i, corte = %r.corte, tipodoc = %r.tipodoc, numdoc = %r.numdoc,
                   deleted = r.deleted_in_dbf, "dbf_reader: caja leído");
        }
    }

    tracing::info!(count = result.len(), "dbf_reader: caja.dbf completo");
    Ok(result)
}
