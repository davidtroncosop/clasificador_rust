use std::fs;
use std::path::{Path, PathBuf};
use regex::Regex;
use crate::config::{MAX_RUTA_WINDOWS, get_aspecto_name, get_foco_name};

pub fn ruta_larga(ruta: &str) -> String {
    if ruta.starts_with(r"\\?\") {
        return ruta.to_string();
    }
    let ruta_abs = Path::new(ruta).canonicalize()
        .unwrap_or_else(|_| Path::new(ruta).to_path_buf());
    
    let ruta_str = ruta_abs.to_string_lossy().to_string();
    
    if ruta_str.starts_with(r"\\") {
        format!(r"\\?\UNC\{}", &ruta_str[2:])
    } else {
        format!(r"\\?\{}", ruta_str)
    }
}

pub fn limpiar_nombre_carpeta(nombre: &str) -> String {
    let chars_invalidos = ['<', '>', ':', '"', '/', '\\', '|', '?', '*'];
    let mut resultado = nombre.to_string();
    for c in chars_invalidos {
        resultado = resultado.replace(c, "");
    }
    resultado.trim_matches(|c| c == '.' || c == ' ').to_string()
}

pub fn acortar_segmento(segmento: &str, max_len: usize) -> String {
    if segmento.len() <= max_len {
        return segmento.to_string();
    }
    
    let re = Regex::new(r"^(\d+\s*-\s*)").unwrap();
    let prefijo = re.captures(segmento)
        .map(|c| c.get(1).unwrap().as_str())
        .unwrap_or("");
    
    let resto = &segmento[prefijo.len()..];
    let chars_disponibles = max_len - prefijo.len() - 1;
    
    format!("{}{}…", prefijo, &resto[..chars_disponibles.min(resto.len())])
}

pub fn construir_ruta_segura(base: &str, segmentos: &[&str]) -> String {
    let segmentos_limpios: Vec<String> = segmentos.iter()
        .map(|s| acortar_segmento(&limpiar_nombre_carpeta(s), 60))
        .collect();
    
    let mut ruta = PathBuf::from(base);
    for seg in &segmentos_limpios {
        ruta = ruta.join(seg);
    }
    
    let mut ruta_str = ruta.to_string_lossy().to_string();
    
    while ruta_str.len() > MAX_RUTA_WINDOWS && !segmentos_limpios.is_empty() {
        let mut max_idx = 0;
        let mut max_len = 0;
        for (i, seg) in segmentos_limpios.iter().enumerate() {
            if seg.len() > max_len {
                max_len = seg.len();
                max_idx = i;
            }
        }
        
        let nuevo_max = std::cmp::max(10, max_len - 10);
        let segmento_actual = &segmentos_limpios[max_idx];
        let nuevo_segmento = acortar_segmento(segmento_actual, nuevo_max);
        
        let mut nuevos_segmentos = segmentos_limpios.clone();
        nuevos_segmentos[max_idx] = nuevo_segmento;
        
        ruta = PathBuf::from(base);
        for seg in &nuevos_segmentos {
            ruta = ruta.join(seg);
        }
        ruta_str = ruta.to_string_lossy().to_string();
    }
    
    ruta_str
}

pub fn nombre_archivo_seguro(ruta_carpeta: &str, nombre_archivo: &str) -> String {
    let ruta_completa = Path::new(ruta_carpeta).join(nombre_archivo);
    let ruta_completa_str = ruta_completa.to_string_lossy().to_string();
    
    if ruta_completa_str.len() <= MAX_RUTA_WINDOWS {
        return nombre_archivo.to_string();
    }
    
    let path = Path::new(nombre_archivo);
    let nombre_base = path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext = path.extension()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    
    let chars_disponibles = MAX_RUTA_WINDOWS - ruta_carpeta.len() - ext.len() - 2;
    let chars_disponibles = std::cmp::max(5, chars_disponibles);
    
    if ext.is_empty() {
        format!("{}…", &nombre_base[..std::cmp::min(chars_disponibles, nombre_base.len())])
    } else {
        format!("{}….{}", &nombre_base[..std::cmp::min(chars_disponibles - 1, nombre_base.len())], ext)
    }
}

pub fn abreviar_aspecto(aspecto_num: &str) -> String {
    let num = aspecto_num.trim_start_matches('0');
    let nombre = get_aspecto_name(&format!("{:02}", num.parse::<u32>().unwrap_or(0)));
    if nombre.is_empty() {
        return aspecto_num.to_string();
    }
    let nombre_limpio = limpiar_nombre_carpeta(&nombre);
    format!("{:02} - {}", aspecto_num, &nombre_limpio[..std::cmp::min(50, nombre_limpio.len())])
}

pub fn eliminar_carpeta_con_reintentos(ruta: &str, intentos: usize, espera: u64) -> bool {
    for intento in 0..intentos {
        if Path::new(ruta).exists() {
            match fs::remove_dir_all(ruta) {
                Ok(_) => return true,
                Err(e) => {
                    if intento < intentos - 1 {
                        tracing::warn!("Carpeta bloqueada, reintentando en {}s... ({}/{})", 
                            espera, intento + 1, intentos);
                        std::thread::sleep(std::time::Duration::from_secs(espera));
                    } else {
                        tracing::error!("No se pudo eliminar '{}': {}", ruta, e);
                        return false;
                    }
                }
            }
        } else {
            return true;
        }
    }
    false
}

pub fn copiar_archivo_seguro(origen: &str, destino: &str) -> bool {
    let origenes = vec![
        ruta_larga(origen),
        origen.to_string(),
    ];
    
    let destinos = vec![
        ruta_larga(destino),
        destino.to_string(),
    ];
    
    for src in &origenes {
        for dst in &destinos {
            if let Err(e) = fs::copy(src, dst) {
                tracing::debug!("Intento fallido: {} -> {}: {}", src, dst, e);
                continue;
            }
            return true;
        }
    }
    
    false
}

pub fn crear_directorios(ruta: &str) -> std::io::Result<()> {
    fs::create_dir_all(ruta)
}

pub fn listar_archivos(carpeta: &str, extensiones_validas: &[&str]) -> Vec<crate::models::ArchivoProcesar> {
    let mut archivos = Vec::new();
    
    for entry in walkdir::WalkDir::new(carpeta)
        .follow_links(true)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            let nombre = path.file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            
            if nombre.starts_with('.') || nombre.starts_with("~$") {
                continue;
            }
            
            let extension = path.extension()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            
            if extensiones_validas.contains(&extension.as_str()) {
                archivos.push(crate::models::ArchivoProcesar {
                    ruta: path.to_string_lossy().to_string(),
                    nombre,
                    extension,
                });
            }
        }
    }
    
    archivos
}
