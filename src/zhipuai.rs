use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use std::sync::atomic::{AtomicU64, Ordering};
use regex::Regex;
use crate::config::{PROMPT_SISTEMA, ZHIPUAI_API_KEY};
use crate::models::AnalisisResultado;

const ZHIPUAI_API_URL: &str = "https://open.bigmodel.cn/api/paas/v4/chat/completions";

const DELAY_ENTRE_SOLICITUDES_MS: u64 = 2000;
const MAX_REINTENTOS_CONCURRENCIA: usize = 5;
const MAX_REINTENTOS_TOTALES: usize = 3;

static ULTIMA_SOLICITUD: std::sync::Mutex<Option<Instant>> = std::sync::Mutex::new(None);
static CONTADOR_SOLICITUDES: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Serialize)]
struct ZhipuRequest {
    model: String,
    messages: Vec<ZhipuMessage>,
    temperature: f32,
    max_tokens: i32,
}

#[derive(Debug, Serialize)]
struct ZhipuMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ZhipuResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    #[serde(rename = "prompt_tokens")]
    prompt_tokens: Option<i32>,
    #[serde(rename = "completion_tokens")]
    completion_tokens: Option<i32>,
    #[serde(rename = "total_tokens")]
    total_tokens: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaError {
    pub error: QuotaErrorDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaErrorDetail {
    pub message: String,
    pub code: String,
    #[serde(rename = "type")]
    pub error_type: String,
}

pub fn verificar_api() -> Result<String, String> {
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let request = ZhipuRequest {
        model: "glm-4-flash".to_string(),
        messages: vec![ZhipuMessage {
            role: "user".to_string(),
            content: "Hi".to_string(),
        }],
        temperature: 0.1,
        max_tokens: 10,
    };

    let response = client
        .post(ZHIPUAI_API_URL)
        .header("Authorization", format!("Bearer {}", ZHIPUAI_API_KEY))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| e.to_string())?;

    if response.status().is_success() {
        Ok("ZhipuAI API conectada correctamente".to_string())
    } else {
        let status = response.status();
        let text = response.text().unwrap_or_default();
        Err(format!("Error API: {} - {}", status, text))
    }
}

pub fn analizar_evidencia(
    nombre_original: &str,
    texto_documento: &str,
    anio_detectado: Option<&str>,
    _max_reintentos: usize,
) -> Result<AnalisisResultado, ApiError> {
    let nota_anio = anio_detectado
        .map(|a| format!(" El año del documento es {}, úsalo exacto en el campo 'Año'.", a))
        .unwrap_or_default();

    let contenido_truncado = if !texto_documento.is_empty() {
        tracing::debug!("Contenido enviado a la IA: {} caracteres", texto_documento.len());
        format!(
            "Nombre del archivo: '{}'.{}\n\nContenido del documento:\n{}",
            nombre_original,
            nota_anio,
            &texto_documento[..std::cmp::min(texto_documento.len(), 3500)]
        )
    } else {
        tracing::debug!("Sin contenido, clasificando solo por nombre");
        format!("Analiza este archivo: '{}'.{}", nombre_original, nota_anio)
    };

    let client = Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let mut reintentos_concurrencia = 0;
    let mut reintentos_totales = 0;

    loop {
        aplicar_delay_entre_solicitudes();

        match hacer_peticion(&client, &contenido_truncado) {
            Ok(resultado) => {
                let total = CONTADOR_SOLICITUDES.fetch_add(1, Ordering::SeqCst) + 1;
                tracing::debug!("Solicitud #{} completada exitosamente", total);
                return Ok(resultado);
            }
            Err(e) => {
                if e.is_concurrencia() && reintentos_concurrencia < MAX_REINTENTOS_CONCURRENCIA {
                    let espera = calcular_backoff(reintentos_concurrencia);
                    reintentos_concurrencia += 1;
                    tracing::warn!("⚠️ Error de concurrencia (intento {}/{}). Esperando {}s...", 
                        reintentos_concurrencia, MAX_REINTENTOS_CONCURRENCIA, espera);
                    std::thread::sleep(Duration::from_secs(espera));
                    continue;
                }

                if e.is_quota_exceeded() {
                    tracing::error!("💰 Cuota de API agotada. Deteniendo proceso.");
                    return Err(e);
                }

                if reintentos_totales < MAX_REINTENTOS_TOTALES {
                    let espera = calcular_backoff(reintentos_totales);
                    reintentos_totales += 1;
                    tracing::warn!("Intento {}/{} fallido: {}. Esperando {}s...", 
                        reintentos_totales, MAX_REINTENTOS_TOTALES, e, espera);
                    std::thread::sleep(Duration::from_secs(espera));
                    continue;
                }

                tracing::error!("❌ Error después de {} reintentos: {}", 
                    MAX_REINTENTOS_TOTALES + MAX_REINTENTOS_CONCURRENCIA, e);
                return Err(e);
            }
        }
    }
}

fn aplicar_delay_entre_solicitudes() {
    if let Ok(mut ultima) = ULTIMA_SOLICITUD.lock() {
        if let Some(instante) = *ultima {
            let elapsed = instante.elapsed().as_millis() as u64;
            if elapsed < DELAY_ENTRE_SOLICITUDES_MS {
                let espera = DELAY_ENTRE_SOLICITUDES_MS - elapsed;
                tracing::debug!("⏳ Esperando {}ms entre solicitudes (capa gratuita)", espera);
                std::thread::sleep(Duration::from_millis(espera));
            }
        }
        *ultima = Some(Instant::now());
    }
}

fn calcular_backoff(intento: usize) -> u64 {
    let base: u64 = 2;
    let max: u64 = 30;
    let delay = base.pow(intento as u32).min(max);
    delay + (rand_delay() % 3)
}

fn rand_delay() -> u64 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    (nanos % 1000) / 100
}

fn hacer_peticion(client: &Client, mensaje: &str) -> Result<AnalisisResultado, ApiError> {
    let request = ZhipuRequest {
        model: "glm-4-flash".to_string(),
        messages: vec![
            ZhipuMessage {
                role: "system".to_string(),
                content: PROMPT_SISTEMA.to_string(),
            },
            ZhipuMessage {
                role: "user".to_string(),
                content: mensaje.to_string(),
            },
        ],
        temperature: 0.1,
        max_tokens: 2048,
    };

    let response = client
        .post(ZHIPUAI_API_URL)
        .header("Authorization", format!("Bearer {}", ZHIPUAI_API_KEY))
        .header("Content-Type", "application/json")
        .json(&request)
        .send()
        .map_err(|e| ApiError::Network(e.to_string()))?;

    let status = response.status();
    let status_code = status.as_u16();

    if status_code == 402 || status_code == 429 {
        let error_text = response.text().unwrap_or_default();
        
        if error_text.contains("quota") || error_text.contains("insufficient") 
            || error_text.contains("Quota") || error_text.contains("rmb") {
            return Err(ApiError::QuotaExceeded(error_text));
        }

        if status_code == 429 || error_text.contains("too many") 
            || error_text.contains("concurrency") || error_text.contains("rate") {
            return Err(ApiError::Concurrency(error_text));
        }
    }

    if status_code == 400 || status_code == 500 || status_code == 503 {
        let error_text = response.text().unwrap_or_default();
        return Err(ApiError::ApiResponse(status_code, error_text));
    }

    if !status.is_success() {
        let error_text = response.text().unwrap_or_default();
        return Err(ApiError::ApiResponse(status_code, error_text));
    }

    let zhipu_resp: ZhipuResponse = response
        .json()
        .map_err(|e| ApiError::Parse(e.to_string()))?;

    let contenido = zhipu_resp.message.content.trim().to_string();

    let contenido_limpio = limpiar_json_response(&contenido);

    let mut resultado: AnalisisResultado = serde_json::from_str(&contenido_limpio)
        .map_err(|e| ApiError::Parse(format!("JSON inválido: {} - Content: {}", e, contenido_limpio)))?;

    if let Some(foco_num) = extraer_numero(&resultado.foco) {
        resultado.foco = foco_num;
    }

    if let Some(asp_num) = extraer_numero_2digitos(&resultado.aspecto) {
        resultado.aspecto = asp_num;
    }

    Ok(resultado)
}

fn limpiar_json_response(contenido: &str) -> String {
    let mut result = contenido.to_string();

    for fence in &["```json", "```"] {
        if result.starts_with(fence) {
            result = result[fence.len()..].trim().to_string();
        }
    }

    if result.ends_with("```") {
        result = result[..result.len() - 3].trim().to_string();
    }

    let re = Regex::new(r"\{.*\}").ok();
    if let Some(regex) = re {
        if let Some(caps) = regex.captures(&result) {
            result = caps.get(0).unwrap().as_str().to_string();
        }
    }

    result.trim().to_string()
}

fn extraer_numero(texto: &str) -> Option<String> {
    let re = Regex::new(r"(\d)").ok()?;
    re.captures(texto)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn extraer_numero_2digitos(texto: &str) -> Option<String> {
    let re = Regex::new(r"(\d{1,2})").ok()?;
    re.captures(texto)
        .and_then(|c| c.get(1))
        .map(|m| {
            let num: u32 = m.as_str().parse().unwrap_or(0);
            format!("{:02}", num)
        })
}

#[derive(Debug)]
pub enum ApiError {
    QuotaExceeded(String),
    Concurrency(String),
    Network(String),
    ApiResponse(u16, String),
    Parse(String),
}

impl ApiError {
    pub fn is_quota_exceeded(&self) -> bool {
        matches!(self, ApiError::QuotaExceeded(_))
    }

    pub fn is_concurrencia(&self) -> bool {
        matches!(self, ApiError::Concurrency(_))
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::QuotaExceeded(msg) => write!(f, "Cuota agotada: {}", msg),
            ApiError::Concurrency(msg) => write!(f, "Error de concurrencia: {}", msg),
            ApiError::Network(msg) => write!(f, "Error de red: {}", msg),
            ApiError::ApiResponse(code, msg) => write!(f, "Error API {}: {}", code, msg),
            ApiError::Parse(msg) => write!(f, "Error de parseo: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}
