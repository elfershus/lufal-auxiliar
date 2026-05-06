use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub grpc_endpoint: String,
    pub api_key: String,
    #[serde(default)]
    pub dbf_arts: Option<String>,
    #[serde(default)]
    pub dbf_unidades: Option<String>,
}

impl AppConfig {
    /// Carga la configuración desde:
    ///   1. Variables de entorno GRPC_ENDPOINT y API_KEY (desarrollo)
    ///   2. %APPDATA%\lufal-auxiliar-desktop\config.toml (producción)
    pub fn load() -> Result<Self> {
        // Override por env vars (útil en desarrollo)
        if let (Ok(ep), Ok(key)) = (
            std::env::var("GRPC_ENDPOINT"),
            std::env::var("API_KEY"),
        ) {
            return Ok(AppConfig {
                grpc_endpoint: ep,
                api_key: key,
                dbf_arts: std::env::var("DBF_ARTS").ok(),
                dbf_unidades: std::env::var("DBF_UNIDADES").ok(),
            });
        }

        let config_path = Self::config_path()
            .context("No se pudo determinar el directorio de configuración")?;

        let raw = std::fs::read_to_string(&config_path).with_context(|| {
            format!(
                "No se encontró config.toml en {:?}\n\
                Crea el archivo con:\n\
                  grpc_endpoint = \"https://tu-servidor:443\"\n\
                  api_key       = \"tu-api-key\"",
                config_path
            )
        })?;

        toml::from_str(&raw).context("config.toml tiene formato inválido")
    }

    pub fn save(grpc_endpoint: &str, api_key: &str) -> Result<()> {
        let config_path = Self::config_path()
            .context("No se pudo determinar el directorio de configuración")?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("No se pudo crear el directorio de configuración")?;
        }

        let content = format!(
            "grpc_endpoint = \"{}\"\napi_key       = \"{}\"\n",
            grpc_endpoint, api_key
        );

        std::fs::write(&config_path, content)
            .with_context(|| format!("No se pudo escribir {:?}", config_path))?;

        Ok(())
    }

    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("lufal-auxiliar-desktop").join("config.toml"))
    }

    /// Actualiza un campo DBF en el TOML preservando todos los demás campos.
    pub fn update_field(key: &str, value: &str) -> Result<()> {
        let config_path = Self::config_path()
            .context("No se pudo determinar el directorio de configuración")?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("No se pudo crear el directorio de configuración")?;
        }

        let existing = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .ok()
                .and_then(|raw| toml::from_str::<toml::Value>(&raw).ok())
                .and_then(|v| v.as_table().cloned())
        } else {
            None
        };

        let grpc_endpoint = existing
            .as_ref()
            .and_then(|t| t.get("grpc_endpoint"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let api_key = existing
            .as_ref()
            .and_then(|t| t.get("api_key"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let dbf_arts = if key == "dbf_arts" {
            value.to_string()
        } else {
            existing
                .as_ref()
                .and_then(|t| t.get("dbf_arts"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let dbf_unidades = if key == "dbf_unidades" {
            value.to_string()
        } else {
            existing
                .as_ref()
                .and_then(|t| t.get("dbf_unidades"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };

        let mut content = format!(
            "grpc_endpoint = \"{}\"\napi_key       = \"{}\"\n",
            grpc_endpoint, api_key
        );

        if !dbf_arts.is_empty() {
            // Escape backslashes for TOML
            let escaped = dbf_arts.replace('\\', "\\\\");
            content.push_str(&format!("dbf_arts      = \"{}\"\n", escaped));
        }
        if !dbf_unidades.is_empty() {
            let escaped = dbf_unidades.replace('\\', "\\\\");
            content.push_str(&format!("dbf_unidades  = \"{}\"\n", escaped));
        }

        std::fs::write(&config_path, content)
            .with_context(|| format!("No se pudo escribir {:?}", config_path))?;

        Ok(())
    }
}
