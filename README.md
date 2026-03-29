# Clasificador de Documentos con IA

Clasificador automático de documentos académicos usando la API de ZhipuAI (GLM-4-Flash).

## Características

- 📄 **Extracción de texto**: PDF, DOC, DOCX
- 🤖 **Clasificación con IA**: GLM-4-Flash
- 💾 **Sistema de checkpoints**: Continúa si se interrumpe
- 💰 **Manejo de cuota**: Se detiene gracefully si se agota la API
- 📊 **Generación automática**: Excel + ZIP clasificado

## Requisitos

- Rust 1.70+
- API Key de ZhipuAI
- antiword (opcional, para archivos .doc)

## Instalación

```bash
# Instalar Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Compilar
cargo build --release
```

## Configuración

Edita `src/config.rs` para cambiar:
- API Key de ZhipuAI
- Rutas de entrada/salida
- Mapas de clasificación

## Uso

```bash
# Ejecutar
./target/release/clasificador_rust
```

## Estructura de Salida

```
salida/
├── Matriz_Revisión_Pares.xlsx
├── Evidencias_Clasificadas_Pares.zip
└── [Programa]/
    └── FOCO X/
        └── [01-13] - [Aspecto]/
            └── [Año]/
                └── archivo.pdf
```

## Licencia

MIT
