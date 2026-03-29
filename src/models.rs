use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalisisResultado {
    #[serde(rename = "Programa")]
    pub programa: String,
    #[serde(rename = "Modalidad")]
    pub modalidad: String,
    #[serde(rename = "Foco")]
    pub foco: String,
    #[serde(rename = "Aspecto")]
    pub aspecto: String,
    #[serde(rename = "Año")]
    pub anio: String,
    #[serde(rename = "Pertinencia")]
    pub pertinencia: String,
    #[serde(rename = "Suficiencia")]
    pub suficiencia: String,
    #[serde(skip)]
    pub archivo: String,
    #[serde(skip)]
    pub archivo_destino: String,
    #[serde(skip)]
    pub foco_completo: String,
    #[serde(skip)]
    pub aspecto_completo: String,
    #[serde(skip)]
    pub ruta_relativa: String,
    #[serde(skip)]
    pub anio_detectado: String,
}

impl Default for AnalisisResultado {
    fn default() -> Self {
        Self {
            programa: String::new(),
            modalidad: String::new(),
            foco: String::new(),
            aspecto: String::new(),
            anio: String::new(),
            pertinencia: String::new(),
            suficiencia: String::new(),
            archivo: String::new(),
            archivo_destino: String::new(),
            foco_completo: String::new(),
            aspecto_completo: String::new(),
            ruta_relativa: String::new(),
            anio_detectado: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub messages: Vec<OllamaMessage>,
    pub stream: bool,
    pub options: OllamaOptions,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaOptions {
    pub temperature: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaResponse {
    pub message: OllamaResponseMessage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaResponseMessage {
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ArchivoProcesar {
    pub ruta: String,
    pub nombre: String,
    pub extension: String,
}
