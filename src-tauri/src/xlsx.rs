use crate::types::{
    PairingRow, PairingRowPreview, ParsePairingsResult,
    SeguimientoFraccionRow, SeguimientoFraccionPreview, ParseSeguimientosResult,
};
use anyhow::Result;
use calamine::{open_workbook_auto, Data, Reader};
use rust_xlsxwriter::{Format, Workbook};
use std::collections::HashSet;

fn cell_to_string(cell: &Data) -> String {
    match cell {
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                format!("{f}")
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

pub fn write_template(path: &str) -> Result<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Emparejamientos")?;

    let bold = Format::new().set_bold();
    ws.write_with_format(0, 0, "numart_origen", &bold)?;
    ws.write_with_format(0, 1, "unidad_fraccion", &bold)?;
    ws.write_with_format(0, 2, "numart_destino", &bold)?;

    ws.write(1, 0, "PROD001")?;
    ws.write(1, 1, "PQTE")?;
    ws.write(1, 2, "PROD002")?;

    ws.set_column_width(0, 22.0)?;
    ws.set_column_width(1, 20.0)?;
    ws.set_column_width(2, 22.0)?;

    wb.save(path)?;
    Ok(())
}

pub fn write_pairings(path: &str, rows: &[PairingRow]) -> Result<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Emparejamientos")?;

    let bold = Format::new().set_bold();
    ws.write_with_format(0, 0, "numart_origen", &bold)?;
    ws.write_with_format(0, 1, "unidad_fraccion", &bold)?;
    ws.write_with_format(0, 2, "numart_destino", &bold)?;

    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, r.numart_origen.as_str())?;
        ws.write(row, 1, r.unidad_fraccion.as_str())?;
        ws.write(row, 2, r.numart_destino.as_str())?;
    }

    ws.set_column_width(0, 22.0)?;
    ws.set_column_width(1, 20.0)?;
    ws.set_column_width(2, 22.0)?;

    wb.save(path)?;
    Ok(())
}

pub fn write_seguimientos_template(path: &str) -> Result<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Seguimientos")?;

    let bold = Format::new().set_bold();
    ws.write_with_format(0, 0, "numart_origen", &bold)?;
    ws.write_with_format(0, 1, "unidad_fraccion", &bold)?;

    ws.write(1, 0, "PROD001")?;
    ws.write(1, 1, "PQTE")?;

    ws.set_column_width(0, 22.0)?;
    ws.set_column_width(1, 20.0)?;

    wb.save(path)?;
    Ok(())
}

pub fn write_seguimientos(path: &str, rows: &[SeguimientoFraccionRow]) -> Result<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name("Seguimientos")?;

    let bold = Format::new().set_bold();
    ws.write_with_format(0, 0, "numart_origen", &bold)?;
    ws.write_with_format(0, 1, "unidad_fraccion", &bold)?;

    for (i, r) in rows.iter().enumerate() {
        let row = (i + 1) as u32;
        ws.write(row, 0, r.numart_origen.as_str())?;
        ws.write(row, 1, r.unidad_fraccion.as_str())?;
    }

    ws.set_column_width(0, 22.0)?;
    ws.set_column_width(1, 20.0)?;

    wb.save(path)?;
    Ok(())
}

pub fn parse_seguimientos(path: &str) -> Result<ParseSeguimientosResult> {
    let mut wb = open_workbook_auto(path)?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow::anyhow!("El archivo no tiene hojas"))??;

    let mut rows: Vec<SeguimientoFraccionPreview> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut display_index = 0usize;

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 {
            continue;
        }

        let get = |col: usize| -> String {
            row.get(col).map(cell_to_string).unwrap_or_default()
        };

        let numart_origen = get(0);
        let unidad_fraccion = get(1);

        if numart_origen.is_empty() && unidad_fraccion.is_empty() {
            continue;
        }

        display_index += 1;
        let mut errors: Vec<String> = Vec::new();

        if numart_origen.is_empty() {
            errors.push("numart_origen vacío".to_string());
        } else if numart_origen.len() > 50 {
            errors.push("numart_origen excede 50 caracteres".to_string());
        }

        if unidad_fraccion.is_empty() {
            errors.push("unidad_fraccion vacía".to_string());
        } else if unidad_fraccion.len() > 50 {
            errors.push("unidad_fraccion excede 50 caracteres".to_string());
        }

        if errors.is_empty() {
            let key = (numart_origen.clone(), unidad_fraccion.clone());
            if seen.contains(&key) {
                errors.push("Clave duplicada en el archivo".to_string());
            } else {
                seen.insert(key);
            }
        }

        rows.push(SeguimientoFraccionPreview {
            row_index: display_index,
            numart_origen,
            unidad_fraccion,
            errors,
        });
    }

    let total_rows = rows.len();
    let error_count = rows.iter().filter(|r| !r.errors.is_empty()).count();
    let valid_count = total_rows - error_count;

    Ok(ParseSeguimientosResult { rows, total_rows, valid_count, error_count })
}

pub fn parse_pairings(path: &str) -> Result<ParsePairingsResult> {
    let mut wb = open_workbook_auto(path)?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| anyhow::anyhow!("El archivo no tiene hojas"))??;

    let mut rows: Vec<PairingRowPreview> = Vec::new();
    let mut seen: HashSet<(String, String)> = HashSet::new();
    let mut display_index = 0usize;

    for (row_idx, row) in range.rows().enumerate() {
        if row_idx == 0 {
            continue;
        }

        let get = |col: usize| -> String {
            row.get(col).map(cell_to_string).unwrap_or_default()
        };

        let numart_origen = get(0);
        let unidad_fraccion = get(1);
        let numart_destino = get(2);

        if numart_origen.is_empty() && unidad_fraccion.is_empty() && numart_destino.is_empty() {
            continue;
        }

        display_index += 1;
        let mut errors: Vec<String> = Vec::new();

        if numart_origen.is_empty() {
            errors.push("numart_origen vacío".to_string());
        } else if numart_origen.len() > 50 {
            errors.push("numart_origen excede 50 caracteres".to_string());
        }

        if unidad_fraccion.is_empty() {
            errors.push("unidad_fraccion vacía".to_string());
        } else if unidad_fraccion.len() > 50 {
            errors.push("unidad_fraccion excede 50 caracteres".to_string());
        }

        if numart_destino.is_empty() {
            errors.push("numart_destino vacío".to_string());
        } else if numart_destino.len() > 50 {
            errors.push("numart_destino excede 50 caracteres".to_string());
        }

        if errors.is_empty() {
            let key = (numart_origen.clone(), unidad_fraccion.clone());
            if seen.contains(&key) {
                errors.push("Clave duplicada en el archivo".to_string());
            } else {
                seen.insert(key);
            }
        }

        rows.push(PairingRowPreview {
            row_index: display_index,
            numart_origen,
            unidad_fraccion,
            numart_destino,
            errors,
        });
    }

    let total_rows = rows.len();
    let error_count = rows.iter().filter(|r| !r.errors.is_empty()).count();
    let valid_count = total_rows - error_count;

    Ok(ParsePairingsResult { rows, total_rows, valid_count, error_count })
}
