pub const MAX_CHARS: usize = 4000;
pub const MAX_RUTA_WINDOWS: usize = 255;

pub static MAPA_ASPECTOS: &[(&str, &str)] = &[
    ("01", "Actualización de currículum"),
    ("02", "Instancias de Retroalimentación"),
    ("03", "Formación teórica-práctica (Pregrado)"),
    ("04", "Proceso de graduación (Posgrados)"),
    ("05", "Proceso de titulación Especialidades de la salud"),
    ("06", "Aplicación de mecanismos de verificación del cumplimiento del perfil de egreso/grado"),
    ("07", "Monitoreo y evaluación de resultados de progresión comparados"),
    ("08", "Seguimiento de egresados/graduados"),
    ("09", "Dotación adecuada y equivalente según número de estudiantes y sedes"),
    ("10", "Calificación y pertinencia del cuerpo académico"),
    ("11", "Desarrollo docente"),
    ("12", "Disponibilidad de los recursos operativos"),
    ("13", "Disponibilidad de los recursos económicos"),
];

pub static MAPA_FOCOS: &[(&str, &str)] = &[
    ("1", "FOCO 1. DISEÑO Y ACTUALIZACIÓN CURRICULAR: PERFIL DE EGRESO Y PLAN DE ESTUDIO"),
    ("2", "FOCO 2. PROCESO Y RESULTADOS DE ENSEÑANZA-APRENDIZAJE CONDUCENTES AL LOGRO DEL PERFIL DE EGRESO"),
    ("3", "FOCO 3. CUERPO ACADÉMICO O DOCENTE"),
    ("4", "FOCO 4. RECURSOS OPERATIVOS Y ECONÓMICOS"),
];

pub fn get_aspecto_name(codigo: &str) -> String {
    MAPA_ASPECTOS
        .iter()
        .find(|(k, _)| *k == codigo)
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| codigo.to_string())
}

pub fn get_foco_name(codigo: &str) -> String {
    MAPA_FOCOS
        .iter()
        .find(|(k, _)| *k == codigo)
        .map(|(_, v)| v.to_string())
        .unwrap_or_else(|| format!("FOCO {}", codigo))
}

pub fn get_programas() -> Vec<&'static str> {
    vec![
        "KINESIOLOGIA",
        "FONOAUDIOLOGIA",
        "NUTRICION Y DIETETICA",
        "LICENCIATURA EN CIENCIAS DE LA ACTIVIDAD FISICA",
        "MAGISTER CLINICO EN AUDIOLOGIA Y EQUILIBRIO",
        "MAGISTER EN KINESIOLOGIA MUSCULOESQUELETICA",
        "MAGISTER EN NEUROKINESIOLOGIA",
        "MAGISTER EN NUTRICION EN SALUD PUBLICA",
        "TERAPIA OCUPACIONAL",
        "MAGISTER INTERDISCIPLINARIO EN NEURODESARROLLO Y NEUROEDUCACION",
    ]
}

pub fn get_modalidades() -> Vec<&'static str> {
    vec!["Presencial", "Semipresencial", "No presencial"]
}

pub const ZHIPUAI_API_KEY: &str = "c15343ebb1f34b6e9714ce067cd9da6d.KljOqngxInIbjlaN";

pub static PROMPT_SISTEMA: &str = r#"Eres un auditor académico experto. Clasifica el documento según las siguientes listas.
Tienes acceso al nombre del archivo Y al contenido real del documento para clasificar con mayor precisión.

PROGRAMAS: KINESIOLOGIA, FONOAUDIOLOGIA, NUTRICION Y DIETETICA, LICENCIATURA EN CIENCIAS DE LA ACTIVIDAD FISICA, MAGISTER CLINICO EN AUDIOLOGIA Y EQUILIBRIO, MAGISTER EN KINESIOLOGIA MUSCULOESQUELETICA, MAGISTER EN NEUROKINESIOLOGIA, MAGISTER EN NUTRICION EN SALUD PUBLICA, TERAPIA OCUPACIONAL, MAGISTER INTERDISCIPLINARIO EN NEURODESARROLLO Y NEUROEDUCACION.

MODALIDAD: Presencial, Semipresencial, No presencial.

FOCOS Y ASPECTOS (el Aspecto pertenece al Foco indicado):
  FOCO 1. DISEÑO Y ACTUALIZACIÓN CURRICULAR: PERFIL DE EGRESO Y PLAN DE ESTUDIO
    01. Actualización de currículum
    02. Instancias de Retroalimentación
    03. Formación teórica-práctica (Pregrado)
    04. Proceso de graduación (Posgrados)
    05. Proceso de titulación Especialidades de la salud

  FOCO 2. PROCESO Y RESULTADOS DE ENSEÑANZA-APRENDIZAJE CONDUCENTES AL LOGRO DEL PERFIL DE EGRESO
    06. Aplicación de mecanismos de verificación del cumplimiento del perfil de egreso/grado
    07. Monitoreo y evaluación de resultados de progresión comparados
    08. Seguimiento de egresados/graduados

  FOCO 3. CUERPO ACADÉMICO O DOCENTE
    09. Dotación adecuada y equivalente según número de estudiantes y sedes
    10. Calificación y pertinencia del cuerpo académico
    11. Desarrollo docente

  FOCO 4. RECURSOS OPERATIVOS Y ECONÓMICOS
    12. Disponibilidad de los recursos operativos
    13. Disponibilidad de los recursos económicos

Evalúa basándote en el contenido real del documento:
1. Pertinencia: Por qué el contenido del documento se relaciona con el Aspecto y Foco asignado.
2. Suficiencia: Por qué el contenido basta para demostrar el cumplimiento.

Si se te indica el año del documento, úsalo tal cual sin modificarlo.
Si no se indica, infiere el año desde el contenido o nombre del archivo.

Responde ÚNICAMENTE con un JSON válido con las claves:
"Programa", "Modalidad", "Foco", "Aspecto", "Año", "Pertinencia", "Suficiencia"
- "Foco" debe ser SOLO el número: "1", "2", "3" o "4"
- "Aspecto" debe ser SOLO el número de 2 dígitos: "01", "02" ... "13"
Sin bloques de código, sin texto adicional, solo el JSON."#;
