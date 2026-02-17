use crate::aieos::types::AIEOS;
use openclaw_core::{OpenClawError, Result};

pub struct AIEOSParser;

impl AIEOSParser {
    pub fn from_json(json: &str) -> Result<AIEOS> {
        let aieos: AIEOS = serde_json::from_str(json)
            .map_err(|e| OpenClawError::Config(format!("AIEOS parse error: {}", e)))?;

        Self::validate(&aieos)?;

        Ok(aieos)
    }

    pub fn from_file(path: &std::path::Path) -> Result<AIEOS> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| OpenClawError::Config(format!("Failed to read AIEOS file: {}", e)))?;

        Self::from_json(&content)
    }

    pub fn validate(aieos: &AIEOS) -> Result<()> {
        if aieos.version.is_empty() {
            return Err(OpenClawError::Config(
                "AIEOS version is required".to_string(),
            ));
        }

        if aieos.identity.names.first.is_empty() {
            return Err(OpenClawError::Config(
                "Agent name (identity.names.first) is required".to_string(),
            ));
        }

        if aieos.psychology.neural_matrix.creativity > 1.0
            || aieos.psychology.neural_matrix.creativity < 0.0
        {
            return Err(OpenClawError::Config(
                "Neural matrix values must be between 0 and 1".to_string(),
            ));
        }

        if aieos.linguistics.text_style.formality_level > 1.0
            || aieos.linguistics.text_style.formality_level < 0.0
        {
            return Err(OpenClawError::Config(
                "Formality level must be between 0 and 1".to_string(),
            ));
        }

        if aieos.motivations.core_drive.is_empty() {
            return Err(OpenClawError::Config("Core drive is required".to_string()));
        }

        Ok(())
    }

    pub fn merge_with_defaults(&self, aieos: &AIEOS) -> AIEOS {
        let mut merged = aieos.clone();

        if merged.version.is_empty() {
            merged.version = "1.1".to_string();
        }

        if merged.identity.names.first.is_empty() {
            merged.identity.names.first = "Agent".to_string();
        }

        merged
    }
}
