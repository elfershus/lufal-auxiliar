use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SucursalConfig {
    pub numalm: String,
    pub letra: String,
    #[serde(default)]
    pub dbf_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub grpc_endpoint: String,
    pub api_key: String,
    #[serde(default)]
    pub default_numalm: Option<String>,
    #[serde(default)]
    pub sucursales: Vec<SucursalConfig>,
}

impl AppConfig {
    pub fn dbf_path_for<'a>(&'a self, numalm: &str) -> Option<&'a str> {
        self.sucursales
            .iter()
            .find(|s| s.numalm.trim() == numalm.trim())
            .and_then(|s| s.dbf_path.as_deref())
    }

    pub fn docum_path_for(&self, numalm: &str) -> Option<String> {
        self.dbf_path_for(numalm).map(|p| format!("{}\\Docum.DBF", p))
    }

    pub fn cxc_path_for(&self, numalm: &str) -> Option<String> {
        self.dbf_path_for(numalm).map(|p| format!("{}\\CXC.DBF", p))
    }

    pub fn arts_path_for(&self, numalm: &str) -> Option<String> {
        self.dbf_path_for(numalm).map(|p| format!("{}\\Arts.DBF", p))
    }

    pub fn minv_path_for(&self, numalm: &str) -> Option<String> {
        self.dbf_path_for(numalm).map(|p| format!("{}\\Minv.DBF", p))
    }

    pub fn unidades_path_for(&self, numalm: &str) -> Option<String> {
        self.dbf_path_for(numalm).map(|p| format!("{}\\Unidades.DBF", p))
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        if let (Ok(ep), Ok(key)) = (
            std::env::var("GRPC_ENDPOINT"),
            std::env::var("API_KEY"),
        ) {
            return Ok(AppConfig {
                grpc_endpoint: ep,
                api_key: key,
                default_numalm: std::env::var("DEFAULT_NUMALM").ok(),
                sucursales: vec![],
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

    /// Carga el config existente como tabla TOML (o tabla vacía si no existe).
    fn load_table() -> toml::map::Map<String, toml::Value> {
        let config_path = match Self::config_path() {
            Some(p) => p,
            None => return toml::map::Map::new(),
        };
        if !config_path.exists() {
            return toml::map::Map::new();
        }
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|raw| toml::from_str::<toml::Value>(&raw).ok())
            .and_then(|v| v.as_table().cloned())
            .unwrap_or_default()
    }

    /// Reescribe el TOML completo preservando todos los campos escalares y
    /// la lista de sucursales con sus dbf_path existentes.
    fn write_config(
        grpc_endpoint: &str,
        api_key: &str,
        default_numalm: Option<&str>,
        sucursales: &[SucursalConfig],
    ) -> Result<()> {
        let config_path = Self::config_path()
            .context("No se pudo determinar el directorio de configuración")?;

        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .context("No se pudo crear el directorio de configuración")?;
        }

        let mut content = format!(
            "grpc_endpoint = \"{}\"\napi_key       = \"{}\"\n",
            grpc_endpoint, api_key
        );

        if let Some(dn) = default_numalm {
            if !dn.is_empty() {
                content.push_str(&format!("default_numalm = \"{}\"\n", dn));
            }
        }

        for s in sucursales {
            let letra = s.letra.trim().chars().next().unwrap_or(' ');
            if letra == ' ' {
                continue;
            }
            content.push_str(&format!(
                "\n[[sucursales]]\nnumalm = \"{}\"\nletra  = \"{}\"\n",
                s.numalm.trim(),
                letra
            ));
            if let Some(dp) = &s.dbf_path {
                if !dp.is_empty() {
                    let escaped = dp.replace('\\', "\\\\");
                    content.push_str(&format!("dbf_path = \"{}\"\n", escaped));
                }
            }
        }

        std::fs::write(&config_path, content)
            .with_context(|| format!("No se pudo escribir {:?}", config_path))?;

        Ok(())
    }

    pub fn update_sucursales(entries: &[SucursalConfig]) -> Result<()> {
        let existing = Self::load_table();

        let grpc_endpoint = existing.get("grpc_endpoint")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let api_key = existing.get("api_key")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let default_numalm = existing.get("default_numalm")
            .and_then(|v| v.as_str()).map(|s| s.to_string());

        // Preservar dbf_path existente por numalm
        let existing_sucursales: Vec<SucursalConfig> = existing
            .get("sucursales")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    let t = item.as_table()?;
                    Some(SucursalConfig {
                        numalm: t.get("numalm")?.as_str()?.trim().to_string(),
                        letra: t.get("letra")?.as_str()?.trim().to_string(),
                        dbf_path: t.get("dbf_path")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string()),
                    })
                }).collect()
            })
            .unwrap_or_default();

        // Merge: para cada entrada nueva, preservar dbf_path si no viene en el array
        let merged: Vec<SucursalConfig> = entries.iter().map(|e| {
            let existing_dp = existing_sucursales.iter()
                .find(|ex| ex.numalm.trim() == e.numalm.trim())
                .and_then(|ex| ex.dbf_path.clone());
            SucursalConfig {
                numalm: e.numalm.clone(),
                letra: e.letra.clone(),
                dbf_path: e.dbf_path.clone().or(existing_dp),
            }
        }).collect();

        Self::write_config(
            &grpc_endpoint,
            &api_key,
            default_numalm.as_deref(),
            &merged,
        )
    }

    /// Actualiza la carpeta DBF de una sucursal específica.
    pub fn update_sucursal_dbf_path(numalm: &str, path: &str) -> Result<()> {
        let existing = Self::load_table();

        let grpc_endpoint = existing.get("grpc_endpoint")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let api_key = existing.get("api_key")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let default_numalm = existing.get("default_numalm")
            .and_then(|v| v.as_str()).map(|s| s.to_string());

        let mut sucursales: Vec<SucursalConfig> = existing
            .get("sucursales")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    let t = item.as_table()?;
                    Some(SucursalConfig {
                        numalm: t.get("numalm")?.as_str()?.trim().to_string(),
                        letra: t.get("letra")?.as_str()?.trim().to_string(),
                        dbf_path: t.get("dbf_path")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string()),
                    })
                }).collect()
            })
            .unwrap_or_default();

        if let Some(s) = sucursales.iter_mut().find(|s| s.numalm.trim() == numalm.trim()) {
            s.dbf_path = Some(path.to_string());
        }

        Self::write_config(&grpc_endpoint, &api_key, default_numalm.as_deref(), &sucursales)
    }

    /// Actualiza el almacén predeterminado.
    pub fn update_default_numalm(numalm: &str) -> Result<()> {
        let existing = Self::load_table();

        let grpc_endpoint = existing.get("grpc_endpoint")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let api_key = existing.get("api_key")
            .and_then(|v| v.as_str()).unwrap_or("").to_string();

        let sucursales: Vec<SucursalConfig> = existing
            .get("sucursales")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|item| {
                    let t = item.as_table()?;
                    Some(SucursalConfig {
                        numalm: t.get("numalm")?.as_str()?.trim().to_string(),
                        letra: t.get("letra")?.as_str()?.trim().to_string(),
                        dbf_path: t.get("dbf_path")
                            .and_then(|v| v.as_str())
                            .filter(|s| !s.is_empty())
                            .map(|s| s.to_string()),
                    })
                }).collect()
            })
            .unwrap_or_default();

        Self::write_config(&grpc_endpoint, &api_key, Some(numalm), &sucursales)
    }
}
