use anyhow::{Context, Result};
use rusqlite::Connection;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::types::Etiqueta;

pub fn db_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("lufal-ordenes").join("pairings.db"))
}

pub fn init_db() -> Result<Connection> {
    let path = db_path().context("No se pudo determinar el directorio de configuración")?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .context("No se pudo crear el directorio de la base de datos")?;
    }

    let conn = Connection::open(&path)
        .with_context(|| format!("No se pudo abrir la base de datos en {:?}", path))?;

    conn.execute_batch(
        "PRAGMA foreign_keys = ON;

         CREATE TABLE IF NOT EXISTS fraccion_pairings (
             numart_origen   TEXT NOT NULL,
             unidad_fraccion TEXT NOT NULL,
             numart_destino  TEXT NOT NULL,
             PRIMARY KEY (numart_origen, unidad_fraccion)
         );

         CREATE TABLE IF NOT EXISTS etiquetas (
             id      INTEGER PRIMARY KEY AUTOINCREMENT,
             nombre  TEXT    NOT NULL,
             color   TEXT    NOT NULL
         );

         CREATE TABLE IF NOT EXISTS emparejamiento_etiquetas (
             numart_origen   TEXT    NOT NULL,
             unidad_fraccion TEXT    NOT NULL,
             etiqueta_id     INTEGER NOT NULL,
             PRIMARY KEY (numart_origen, unidad_fraccion, etiqueta_id),
             FOREIGN KEY (etiqueta_id) REFERENCES etiquetas(id) ON DELETE CASCADE
         );",
    )
    .context("No se pudo inicializar la base de datos")?;

    Ok(conn)
}

pub fn get_all_pairings() -> Result<HashMap<(String, String), String>> {
    let conn = init_db()?;
    let mut stmt = conn.prepare(
        "SELECT numart_origen, unidad_fraccion, numart_destino FROM fraccion_pairings",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut map = HashMap::new();
    for row in rows {
        let (origen, unidad, destino) = row?;
        map.insert((origen, unidad), destino);
    }
    Ok(map)
}

pub fn upsert_pairing(numart_origen: &str, unidad_fraccion: &str, numart_destino: &str) -> Result<()> {
    let conn = init_db()?;
    conn.execute(
        "INSERT INTO fraccion_pairings (numart_origen, unidad_fraccion, numart_destino)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(numart_origen, unidad_fraccion) DO UPDATE SET numart_destino = excluded.numart_destino",
        rusqlite::params![numart_origen, unidad_fraccion, numart_destino],
    )?;
    Ok(())
}

pub fn delete_pairing(numart_origen: &str, unidad_fraccion: &str) -> Result<()> {
    let conn = init_db()?;
    conn.execute(
        "DELETE FROM emparejamiento_etiquetas WHERE numart_origen = ?1 AND unidad_fraccion = ?2",
        rusqlite::params![numart_origen, unidad_fraccion],
    )?;
    conn.execute(
        "DELETE FROM fraccion_pairings WHERE numart_origen = ?1 AND unidad_fraccion = ?2",
        rusqlite::params![numart_origen, unidad_fraccion],
    )?;
    Ok(())
}

// ── Etiquetas CRUD ─────────────────────────────────────────────────────────

pub fn get_all_etiquetas() -> Result<Vec<Etiqueta>> {
    let conn = init_db()?;
    let mut stmt = conn.prepare("SELECT id, nombre, color FROM etiquetas ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        Ok(Etiqueta {
            id:     row.get(0)?,
            nombre: row.get(1)?,
            color:  row.get(2)?,
        })
    })?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(anyhow::Error::from)
}

pub fn create_etiqueta(nombre: &str, color: &str) -> Result<Etiqueta> {
    let conn = init_db()?;
    conn.execute(
        "INSERT INTO etiquetas (nombre, color) VALUES (?1, ?2)",
        rusqlite::params![nombre, color],
    )?;
    let id = conn.last_insert_rowid();
    Ok(Etiqueta { id, nombre: nombre.to_string(), color: color.to_string() })
}

pub fn update_etiqueta(id: i64, nombre: &str, color: &str) -> Result<()> {
    let conn = init_db()?;
    conn.execute(
        "UPDATE etiquetas SET nombre = ?1, color = ?2 WHERE id = ?3",
        rusqlite::params![nombre, color, id],
    )?;
    Ok(())
}

pub fn delete_etiqueta(id: i64) -> Result<()> {
    let conn = init_db()?;
    conn.execute("DELETE FROM etiquetas WHERE id = ?1", rusqlite::params![id])?;
    Ok(())
}

// ── Asignación de etiquetas a emparejamientos ──────────────────────────────

pub fn set_pairing_etiquetas(
    numart_origen: &str,
    unidad_fraccion: &str,
    etiqueta_ids: &[i64],
) -> Result<()> {
    let conn = init_db()?;
    conn.execute(
        "DELETE FROM emparejamiento_etiquetas WHERE numart_origen = ?1 AND unidad_fraccion = ?2",
        rusqlite::params![numart_origen, unidad_fraccion],
    )?;
    for &eid in etiqueta_ids {
        conn.execute(
            "INSERT INTO emparejamiento_etiquetas (numart_origen, unidad_fraccion, etiqueta_id)
             VALUES (?1, ?2, ?3)",
            rusqlite::params![numart_origen, unidad_fraccion, eid],
        )?;
    }
    Ok(())
}

pub fn get_all_pairing_etiquetas() -> Result<HashMap<(String, String), Vec<Etiqueta>>> {
    let conn = init_db()?;
    let mut stmt = conn.prepare(
        "SELECT ee.numart_origen, ee.unidad_fraccion, e.id, e.nombre, e.color
         FROM emparejamiento_etiquetas ee
         JOIN etiquetas e ON e.id = ee.etiqueta_id
         ORDER BY ee.numart_origen, ee.unidad_fraccion, e.id",
    )?;
    let mut map: HashMap<(String, String), Vec<Etiqueta>> = HashMap::new();
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            Etiqueta {
                id:     row.get(2)?,
                nombre: row.get(3)?,
                color:  row.get(4)?,
            },
        ))
    })?;
    for row in rows {
        let (origen, unidad, etiqueta) = row?;
        map.entry((origen, unidad)).or_default().push(etiqueta);
    }
    Ok(map)
}
