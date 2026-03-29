mod config;
mod models;
mod file_utils;
mod text_extraction;
mod zhipuai;
mod excel;
mod state;

use std::path::Path;
use std::fs;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

static CUOTA_AGOTADA: AtomicBool = AtomicBool::new(false);

fn main() {
    iniciar_logging();
    
    tracing::info!("Iniciando clasificador de documentos con ZhipuAI");
    
    let carpeta_entrada = obtener_carpeta_entrada();
    let carpeta_salida = obtener_carpeta_salida();
    
    let excel_path = Path::new(&carpeta_salida).join("Matriz_Revisión_Pares.xlsx");
    let zip_base_path = Path::new(&carpeta_salida).join("Evidencias_Clasificadas_Pares");
    let ruta_estado = state::obtener_ruta_estado(&carpeta_salida);
    
    preparar_carpeta_salida(&carpeta_salida);
    
    verificar_api();
    
    let mut archivos = encontrar_archivos(&carpeta_entrada);
    
    if archivos.is_empty() {
        tracing::error!("No se encontraron archivos en: {}", carpeta_entrada);
        std::process::exit(1);
    }
    
    let (datos_excel, errores, archivos_procesados) = 
        cargar_o_preguntar_estado(&ruta_estado, &mut archivos);
    
    let total_a_procesar = archivos.len();
    let total_total = total_a_procesar + archivos_procesados.len();
    
    tracing::info!("{} archivos encontrados ({} ya procesados)", total_total, archivos_procesados.len());
    tracing::info!("Por procesar: {} archivos", total_a_procesar);
    
    let mut estado = state::EstadoProceso {
        archivos_procesados,
        datos_excel,
        errores,
        ultimo_indice: 0,
        checkpoint_contador: 0,
        timestamp_ultimo_guardado: chrono::Utc::now().to_rfc3339(),
    };
    
    let indice_inicio = datos_excel.len();
    let mut datos_excel = datos_excel;
    let mut errores = estado.errores.clone();
    
    for (i, archivo) in archivos.iter().enumerate() {
        if CUOTA_AGOTADA.load(Ordering::SeqCst) {
            tracing::warn!("💰 Cuota de API agotada. Guardando estado para continuar después...");
            if let Err(e) = state::guardar_estado(&ruta_estado, &mut estado) {
                tracing::error!("Error guardando estado final: {}", e);
            }
            generar_salida_final(&datos_excel, &errores, &carpeta_salida, &excel_path, &zip_base_path, &ruta_estado);
            std::process::exit(0);
        }
        
        let numero = indice_inicio + i + 1;
        
        tracing::info!("[{} / {}] Procesando: {}", numero, total_total, archivo.nombre);
        
        let anio_detectado = text_extraction::extraer_anio(&archivo.ruta, &archivo.extension);
        
        if let Some(anio) = &anio_detectado {
            tracing::debug!("Año detectado: {}", anio);
        }
        
        let texto_doc = text_extraction::extraer_texto(&archivo.ruta, &archivo.extension);
        
        match zhipuai::analizar_evidencia(
            &archivo.nombre,
            &texto_doc,
            anio_detectado.as_deref(),
            3,
        ) {
            Ok(resultado) => {
                let foco_num = resultado.foco.clone();
                let aspecto_num = resultado.aspecto.clone();
                let programa = file_utils::limpiar_nombre_carpeta(&resultado.programa);
                let anio = file_utils::limpiar_nombre_carpeta(&resultado.anio);
                
                let ruta_nueva_carpeta = file_utils::construir_ruta_segura(
                    &carpeta_salida,
                    &[
                        &programa,
                        &format!("FOCO {}", foco_num),
                        &file_utils::abreviar_aspecto(&aspecto_num),
                        &anio,
                    ],
                );
                
                if let Err(e) = fs::create_dir_all(&ruta_nueva_carpeta) {
                    tracing::error!("No se pudo crear directorio: {}", e);
                }
                
                let nombre_seguro = file_utils::nombre_archivo_seguro(&ruta_nueva_carpeta, &archivo.nombre);
                let ruta_final = Path::new(&ruta_nueva_carpeta).join(&nombre_seguro);
                
                let copiado = file_utils::copiar_archivo_seguro(&archivo.ruta, &ruta_final.to_string_lossy());
                
                if copiado {
                    let ruta_relativa = ruta_final.to_string_lossy()
                        .strip_prefix(&carpeta_salida)
                        .unwrap_or("")
                        .trim_start_matches(['/', '\\']);
                    
                    let mut resultado_final = resultado;
                    resultado_final.archivo = archivo.nombre.clone();
                    resultado_final.archivo_destino = nombre_seguro;
                    resultado_final.foco_completo = config::get_foco_name(&foco_num);
                    resultado_final.aspecto_completo = config::get_aspecto_name(&aspecto_num);
                    resultado_final.ruta_relativa = ruta_relativa.to_string();
                    resultado_final.anio_detectado = anio_detectado.clone().unwrap_or_else(|| "Inferido por IA".to_string());
                    
                    datos_excel.push(resultado_final.clone());
                    estado.datos_excel.push(resultado_final);
                    estado.archivos_procesados.push(archivo.ruta.clone());
                    
                    tracing::info!("   ✅ -> {}", ruta_relativa);
                } else {
                    errores.push(archivo.nombre.clone());
                    estado.errores.push(archivo.nombre.clone());
                    tracing::warn!("   ⚠️  Omitido por error de copia");
                }
            }
            Err(e) => {
                if e.is_quota_exceeded() {
                    CUOTA_AGOTADA.store(true, Ordering::SeqCst);
                    tracing::error!("💰 Cuota de API agotada: {}", e);
                    continue;
                }
                errores.push(archivo.nombre.clone());
                estado.errores.push(archivo.nombre.clone());
                tracing::warn!("   ⚠️  Omitido (error): {}", e);
            }
        }
    }
    
    generar_salida_final(&datos_excel, &errores, &carpeta_salida, &excel_path, &zip_base_path, &ruta_estado);
}

fn generar_salida_final(
    datos_excel: &[models::AnalisisResultado],
    errores: &[String],
    carpeta_salida: &str,
    excel_path: &Path,
    zip_base_path: &Path,
    ruta_estado: &str,
) {
    if !datos_excel.is_empty() {
        tracing::info!("Generando Excel matriz...");
        
        if let Err(e) = excel::generar_excel(datos_excel, &excel_path.to_string_lossy()) {
            tracing::error!("Error al generar Excel: {}", e);
        } else {
            tracing::info!("Excel guardado en: {}", excel_path.display());
        }
        
        tracing::info!("Comprimiendo estructura final...");
        crear_zip(carpeta_salida, &zip_base_path.to_string_lossy());
        
        if state::eliminar_estado(ruta_estado) {
            tracing::info!("Estado de checkpoint eliminado");
        }
        
        tracing::info!("Proceso completado!");
        tracing::info!("   Archivos clasificados: {}", datos_excel.len());
        tracing::info!("   Archivos con error: {}", errores.len());
        
        if !errores.is_empty() {
            tracing::warn!("Archivos omitidos:");
            for e in errores {
                tracing::warn!("   - {}", e);
            }
        }
    } else {
        tracing::warn!("No se pudo procesar ningún archivo.");
    }
}

fn cargar_o_preguntar_estado(
    ruta_estado: &str,
    archivos: &mut Vec<models::ArchivoProcesar>,
) -> (Vec<models::AnalisisResultado>, Vec<String>, Vec<String>) {
    if let Some(estado_cargado) = state::cargar_estado(ruta_estado) {
        let ya_procesados = estado_cargado.archivos_procesados.len();
        
        if ya_procesados > 0 {
            println!("\n🎯 Se encontró un estado previo:");
            println!("   • Archivos ya procesados: {}", ya_procesados);
            println!("   • Archivos clasificados: {}", estado_cargado.datos_excel.len());
            println!("   • Errores previos: {}", estado_cargado.errores.len());
            println!("   • Última actualización: {}", estado_cargado.timestamp_ultimo_guardado);
            println!("\n¿Qué deseas hacer?");
            println!("   1) Continuar desde donde lo dejaste");
            println!("   2) Empezar desde cero (borra el progreso)");
            
            let opcion = leer_opcion_usuario();
            
            match opcion {
                1 | _ => {
                    if opcion == 1 {
                        println!("\n▶️  Continuando desde el checkpoint...");
                    } else {
                        println!("Opción inválida, continuando desde el checkpoint...");
                    }
                    
                    let rutas_procesadas: std::collections::HashSet<String> = 
                        estado_cargado.archivos_procesados.iter().cloned().collect();
                    
                    archivos.retain(|a| !rutas_procesadas.contains(&a.ruta));
                    
                    return (
                        estado_cargado.datos_excel,
                        estado_cargado.errores,
                        estado_cargado.archivos_procesados,
                    );
                }
                2 => {
                    println!("\n🗑️  Borrando estado y empezando de cero...");
                    let _ = state::eliminar_estado(ruta_estado);
                    return (Vec::new(), Vec::new(), Vec::new());
                }
            }
        }
    }
    
    (Vec::new(), Vec::new(), Vec::new())
}

fn leer_opcion_usuario() -> usize {
    print!("\n> Ingresa tu opción (1-2): ");
    io::stdout().flush().ok();
    
    let mut opcion = String::new();
    if io::stdin().read_line(&mut opcion).ok() {
        opcion.trim().parse().unwrap_or(1)
    } else {
        1
    }
}

fn iniciar_logging() {
    let log_dir = std::env::temp_dir().join("clasificador_rust");
    let _ = fs::create_dir_all(&log_dir);
    
    let file_appender = tracing_appender::rolling::daily(&log_dir, "clasificador.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
        )
        .init();
}

fn obtener_carpeta_entrada() -> String {
    #[cfg(windows)]
    {
        r"C:\Users\david.troncoso\Universidad San Sebastian\Facultad RCV - General\03  Escuela KINE".to_string()
    }
    #[cfg(not(windows))]
    {
        std::env::var("CARPETA_ENTRADA")
            .unwrap_or_else(|_| "./entrada".to_string())
    }
}

fn obtener_carpeta_salida() -> String {
    #[cfg(windows)]
    {
        r"C:\Clasificador_Salida".to_string()
    }
    #[cfg(not(windows))]
    {
        std::env::var("CARPETA_SALIDA")
            .unwrap_or_else(|_| "./salida".to_string())
    }
}

fn preparar_carpeta_salida(carpeta: &str) {
    let path = Path::new(carpeta);
    
    if path.exists() {
        tracing::info!("Limpiando carpeta de salida...");
        file_utils::eliminar_carpeta_con_reintentos(carpeta, 5, 2);
    }
    
    if let Err(e) = fs::create_dir_all(carpeta) {
        tracing::error!("No se pudo crear la carpeta de salida: {}", e);
        std::process::exit(1);
    }
}

fn verificar_api() {
    match zhipuai::verificar_api() {
        Ok(msg) => tracing::info!("🤖 {}", msg),
        Err(e) => {
            tracing::error!("❌ Error conectando a ZhipuAI: {}", e);
            std::process::exit(1);
        }
    }
}

fn encontrar_archivos(carpeta: &str) -> Vec<models::ArchivoProcesar> {
    let extensiones_validas = vec!["pdf", "doc", "docx"];
    file_utils::listar_archivos(carpeta, &extensiones_validas)
}

fn crear_zip(carpeta_base: &str, zip_path: &str) {
    use std::fs::File;
    use zip::write::FileOptions;
    use zip::ZipWriter;
    
    let zip_file = match File::create(format!("{}.zip", zip_path)) {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("No se pudo crear el ZIP: {}", e);
            return;
        }
    };
    
    let mut zip = ZipWriter::new(zip_file);
    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    
    let base_path = Path::new(carpeta_base);
    
    fn agregar_archivos(
        zip: &mut ZipWriter<File>,
        base_path: &Path,
        current_path: &Path,
        options: FileOptions,
    ) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(current_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                agregar_archivos(zip, base_path, &path, options)?;
            } else {
                let relative = path.strip_prefix(base_path)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace("\\", "/");
                
                zip.start_file(&relative, options)?;
                let mut file = File::open(&path)?;
                std::io::copy(&mut file, zip)?;
            }
        }
        Ok(())
    }
    
    if let Err(e) = agregar_archivos(&mut zip, base_path, base_path, options) {
        tracing::error!("Error al agregar archivos al ZIP: {}", e);
    } else {
        if let Err(e) = zip.finish() {
            tracing::error!("Error al finalizar ZIP: {}", e);
        } else {
            tracing::info!("ZIP guardado en: {}.zip", zip_path);
        }
    }
}
