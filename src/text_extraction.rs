use regex::Regex;
use std::fs::File;
use std::io::Read;
use std::sync::LazyLock;
use crate::config::MAX_CHARS;
use crate::file_utils::ruta_larga;

static REGEX_ANIO_NOMBRE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\((\d{2,4})-(\d{2})-(\d{2})\)").unwrap()
});

static REGEX_ANIO_SIMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(20\d{2})").unwrap()
});

static REGEX_ANIO_PDF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"D:(\d{4})").unwrap()
});

pub fn extraer_texto(ruta_archivo: &str, extension: &str) -> String {
    let rutas: Vec<String> = vec![
        ruta_larga(ruta_archivo),
        ruta_archivo.to_string(),
    ];
    
    match extension {
        "pdf" => {
            for ruta in &rutas {
                if let Ok(texto) = extraer_texto_pdf(ruta) {
                    if !texto.is_empty() {
                        return texto;
                    }
                }
            }
        }
        "doc" => {
            for ruta in &rutas {
                if let Ok(texto) = extraer_texto_doc_antiword(ruta) {
                    if !texto.is_empty() {
                        return texto;
                    }
                }
            }
        }
        "docx" => {
            for ruta in &rutas {
                if let Ok(texto) = extraer_texto_docx_moderno(ruta) {
                    if !texto.is_empty() {
                        return texto;
                    }
                }
            }
        }
        _ => {}
    }
    
    String::new()
}

fn extraer_texto_pdf(ruta: &str) -> Result<String, Box<dyn std::error::Error>> {
    let doc = lopdf::Document::load(ruta)?;
    let mut texto = String::new();
    
    for page_num in 1..=doc.get_pages().len() as u32 {
        if texto.len() >= MAX_CHARS {
            break;
        }
        if let Ok(page_text) = doc.extract_text(&[page_num]) {
            texto.push_str(&page_text);
        }
    }
    
    Ok(texto[..std::cmp::min(texto.len(), MAX_CHARS)].to_string())
}

fn extraer_texto_docx(ruta: &str) -> Result<String, Box<dyn std::error::Error>> {
    if let Ok(texto) = extraer_texto_docx_moderno(ruta) {
        if !texto.is_empty() {
            return Ok(texto);
        }
    }
    extraer_texto_doc_antiword(ruta)
}

fn extraer_texto_docx_moderno(ruta: &str) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(ruta)?;
    let reader = docx_rs::DocxReader::new(file)
        .ok()
        .ok_or("No se pudo leer el DOCX")?;
    
    let doc = reader.read()?;
    let mut texto = String::new();
    
    fn extract_text_from_element(element: &docx_rs::DocElement, texto: &mut String) {
        match element {
            docx_rs::DocElement::Paragraph(p) => {
                for child in &p.children {
                    if let docx_rs::ParagraphChild::Run(run) = child {
                        for child in &run.children {
                            if let docx_rs::RunChild::Text(t) = child {
                                texto.push_str(&t.text);
                            }
                        }
                    }
                }
                texto.push(' ');
            }
            docx_rs::DocElement::Table(t) => {
                for row in &t.rows {
                    for cell in &row.cells {
                        for child in &cell.children {
                            extract_text_from_element(child, texto);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    
    for element in &doc.document.children {
        extract_text_from_element(element, &mut texto);
    }
    
    Ok(texto[..std::cmp::min(texto.len(), MAX_CHARS)].to_string())
}

fn extraer_texto_doc_antiword(ruta: &str) -> Result<String, Box<dyn std::error::Error>> {
    use std::process::Command;
    
    let antiword_paths = vec![
        "antiword",
        "C:\\antiword\\antiword.exe",
        "C:\\Program Files\\antiword\\antiword.exe",
    ];
    
    for antiword_cmd in &antiword_paths {
        let output = Command::new(antiword_cmd)
            .arg("-m")
            .arg("UTF-8.txt")
            .arg(ruta)
            .output();
        
        if let Ok(output) = output {
            if output.status.success() {
                let texto = String::from_utf8_lossy(&output.stdout).to_string();
                if !texto.trim().is_empty() {
                    return Ok(texto[..std::cmp::min(texto.len(), MAX_CHARS)].to_string());
                }
            }
        }
    }
    
    Err("antiword no disponible".into())
}

pub fn extraer_anio(ruta_archivo: &str, extension: &str) -> Option<String> {
    let nombre = std::path::Path::new(ruta_archivo)
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    
    if extension == "docx" {
        if let Some(anio) = extraer_anio_docx(ruta_archivo) {
            return Some(anio);
        }
    }
    
    if extension == "pdf" {
        if let Some(anio) = extraer_anio_pdf(ruta_archivo) {
            return Some(anio);
        }
    }
    
    if let Some(anio) = extraer_anio_nombre(&nombre) {
        return Some(anio);
    }
    
    None
}

fn extraer_anio_docx(ruta: &str) -> Option<String> {
    let file = match File::open(ruta_larga(ruta)) {
        Ok(f) => f,
        Err(_) => return None,
    };
    
    let reader = match docx_rs::DocxReader::new(file) {
        Ok(r) => r,
        Err(_) => return None,
    };
    
    let doc = match reader.read() {
        Ok(d) => d,
        Err(_) => return None,
    };
    
    if let Some(created) = doc.core_properties.as_ref()?.created() {
        return Some(created.format("%Y").to_string());
    }
    
    None
}

fn extraer_anio_pdf(ruta: &str) -> Option<String> {
    let doc = lopdf::Document::load(ruta_larga(ruta)).ok()?;
    let metadata = doc.trailer.get(b"Info")
        .and_then(|r| doc.get_dictionary(r).ok());
    
    if let Some(info) = metadata {
        if let Ok(creation_date) = info.get(b"CreationDate") {
            if let Ok(date_str) = creation_date.as_string() {
                let date_bytes = date_str.as_bytes();
                if date_bytes.len() >= 6 {
                    if let Some(caps) = REGEX_ANIO_PDF.captures(&String::from_utf8_lossy(date_bytes)) {
                        return Some(caps.get(1)?.as_str().to_string());
                    }
                }
            }
        }
    }
    
    None
}

fn extraer_anio_nombre(nombre: &str) -> Option<String> {
    if let Some(caps) = REGEX_ANIO_NOMBRE.captures(nombre) {
        let anio = caps.get(1)?.as_str();
        return Some(
            if anio.len() == 2 {
                format!("20{}", anio)
            } else {
                anio.to_string()
            }
        );
    }
    
    REGEX_ANIO_SIMPLE.captures(nombre).map(|c| c.get(1)?.as_str().to_string())
}
