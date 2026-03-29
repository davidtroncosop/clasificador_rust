use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::models::AnalisisResultado;

const CHECKPOINT_INTERVAL: usize = 100;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstadoProceso {
    pub archivos_procesados: Vec<String>,
    pub datos_excel: Vec<AnalisisResultado>,
    pub errores: Vec<String>,
    pub ultimo_indice: usize,
    pub checkpoint_contador: usize,
    pub timestamp_ultimo_guardado: String,
}

impl Default for EstadoProceso {
    fn default() -> Self {
        Self {
            archivos_procesados: Vec::new(),
            datos_excel: Vec::new(),
            errores: Vec::new(),
            ultimo_indice: 0,
            checkpoint_contador: 0,
            timestamp_ultimo_guardado: chrono::Utc::now().to_rfc3339(),
        }
    }
}

pub fn obtener_ruta_estado(carpeta_salida: &str) -> String {
    Path::new(carpeta_salida).join("estado_progreso.json")
        .to_string_lossy()
        .to_string()
}

pub fn cargar_estado(ruta: &str) -> Option<EstadoProceso> {
    let contenido = fs::read_to_string(ruta).ok()?;
    serde_json::from_str(&contenido).ok()
}

pub fn guardar_estado(ruta: &str, estado: &mut EstadoProceso) -> Result<(), String> {
    estado.timestamp_ultimo_guardado = chrono::Utc::now().to_rfc3339();
    estado.checkpoint_contador += 1;
    
    let json = serde_json::to_string_pretty(estado)
        .map_err(|e| e.to_string())?;
    
    fs::write(ruta, json)
        .map_err(|e| e.to_string())?;
    
    tracing::info!("Checkpoint guardado (#{} - {} archivos)", 
        estado.checkpoint_contador, estado.datos_excel.len());
    
    Ok(())
}

pub fn debe_guardar_checkpoint(archivos_procesados: usize) -> bool {
    archivos_procesados > 0 && archivos_procesados % CHECKPOINT_INTERVAL == 0
}

pub fn eliminar_estado(ruta: &str) -> bool {
    if Path::new(ruta).exists() {
        fs::remove_file(ruta).is_ok()
    } else {
        true
    }
}

pub const CHECKPOINT_FRECUENCIA: usize = CHECKPOINT_INTERVAL;
