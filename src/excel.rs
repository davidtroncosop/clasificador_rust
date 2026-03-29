use calamine::{Writer, Xlsx, open_workbook};
use std::path::Path;
use crate::models::AnalisisResultado;

pub fn generar_excel(datos: &[AnalisisResultado], ruta: &str) -> Result<(), String> {
    let mut workbook = Xlsx::new();
    let sheet = workbook.add_worksheet("Datos");
    
    let headers = [
        "Archivo",
        "Archivo Destino",
        "Programa",
        "Modalidad",
        "Foco",
        "Foco Completo",
        "Aspecto",
        "Aspecto Completo",
        "Año",
        "Año Detectado",
        "Pertinencia",
        "Suficiencia",
        "Ruta Relativa",
    ];
    
    for (i, header) in headers.iter().enumerate() {
        sheet.write_string(0, i as u32, header, &Default::default())
            .map_err(|e| e.to_string())?;
    }
    
    for (row, dato) in datos.iter().enumerate() {
        let row = row as u32 + 1;
        
        sheet.write_string(row, 0, &dato.archivo, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 1, &dato.archivo_destino, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 2, &dato.programa, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 3, &dato.modalidad, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 4, &dato.foco, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 5, &dato.foco_completo, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 6, &dato.aspecto, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 7, &dato.aspecto_completo, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 8, &dato.anio, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 9, &dato.anio_detectado, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 10, &dato.pertinencia, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 11, &dato.suficiencia, &Default::default())
            .map_err(|e| e.to_string())?;
        sheet.write_string(row, 12, &dato.ruta_relativa, &Default::default())
            .map_err(|e| e.to_string())?;
    }
    
    let path = Path::new(ruta);
    
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    
    std::fs::write(ruta, Vec::new()).map_err(|e| e.to_string())?;
    
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(ruta)
        .map_err(|e| e.to_string())?;
    
    workbook.save(&mut file).map_err(|e| e.to_string())?;
    
    Ok(())
}
