use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIEOS {
    pub version: String,
    pub identity: Identity,
    pub psychology: Psychology,
    pub linguistics: Linguistics,
    pub motivations: Motivations,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub preferences: Preferences,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub names: Names,
    #[serde(default)]
    pub pronouns: Option<Pronouns>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Names {
    pub first: String,
    #[serde(default)]
    pub middle: Option<String>,
    #[serde(default)]
    pub last: Option<String>,
    #[serde(default)]
    pub nickname: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pronouns {
    pub subject: String,
    pub object: String,
    pub possessive: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Psychology {
    pub neural_matrix: NeuralMatrix,
    pub traits: Traits,
    #[serde(default)]
    pub moral_compass: Option<MoralCompass>,
    #[serde(default)]
    pub emotional_model: Option<EmotionalModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralMatrix {
    pub creativity: f64,
    pub logic: f64,
    pub empathy: f64,
    pub courage: f64,
    pub patience: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Traits {
    #[serde(default)]
    pub mbti: Option<String>,
    #[serde(default)]
    pub big_five: Option<BigFive>,
    #[serde(default)]
    pub custom: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigFive {
    pub openness: f64,
    pub conscientiousness: f64,
    pub extraversion: f64,
    pub agreeableness: f64,
    pub neuroticism: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoralCompass {
    pub alignment: String,
    #[serde(default)]
    pub ethics_framework: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalModel {
    pub volatility: f64,
    pub warmth: f64,
    #[serde(default)]
    pub mood_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Linguistics {
    pub text_style: TextStyle,
    #[serde(default)]
    pub speech_patterns: Vec<SpeechPattern>,
    #[serde(default)]
    pub vocabulary: Option<Vocabulary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    pub formality_level: f64,
    pub slang_usage: bool,
    #[serde(default)]
    pub humor_level: Option<f64>,
    #[serde(default)]
    pub technical_level: Option<f64>,
    #[serde(default)]
    pub sentence_length: Option<String>,
    #[serde(default)]
    pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeechPattern {
    pub pattern: String,
    #[serde(default)]
    pub frequency: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vocabulary {
    #[serde(default)]
    pub technical_terms: Vec<String>,
    #[serde(default)]
    pub preferred_words: Vec<String>,
    #[serde(default)]
    pub avoided_words: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Motivations {
    pub core_drive: String,
    #[serde(default)]
    pub goals: Vec<String>,
    #[serde(default)]
    pub fears: Vec<String>,
    #[serde(default)]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub level: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub level: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    #[serde(default)]
    pub communication_style: Option<String>,
    #[serde(default)]
    pub work_style: Option<String>,
    #[serde(default)]
    pub response_format: Option<String>,
}

impl Default for AIEOS {
    fn default() -> Self {
        Self {
            version: "1.1".to_string(),
            identity: Identity::default(),
            psychology: Psychology::default(),
            linguistics: Linguistics::default(),
            motivations: Motivations::default(),
            capabilities: Vec::new(),
            preferences: Preferences::default(),
        }
    }
}

impl Default for Identity {
    fn default() -> Self {
        Self {
            names: Names {
                first: "Agent".to_string(),
                middle: None,
                last: None,
                nickname: None,
            },
            pronouns: None,
            avatar: None,
            role: None,
        }
    }
}

impl Default for Psychology {
    fn default() -> Self {
        Self {
            neural_matrix: NeuralMatrix {
                creativity: 0.5,
                logic: 0.5,
                empathy: 0.5,
                courage: 0.5,
                patience: 0.5,
            },
            traits: Traits::default(),
            moral_compass: None,
            emotional_model: None,
        }
    }
}

impl Default for Traits {
    fn default() -> Self {
        Self {
            mbti: None,
            big_five: None,
            custom: Vec::new(),
        }
    }
}

impl Default for Linguistics {
    fn default() -> Self {
        Self {
            text_style: TextStyle {
                formality_level: 0.5,
                slang_usage: false,
                humor_level: None,
                technical_level: None,
                sentence_length: None,
                tone: None,
            },
            speech_patterns: Vec::new(),
            vocabulary: None,
        }
    }
}

impl Default for Motivations {
    fn default() -> Self {
        Self {
            core_drive: "帮助用户".to_string(),
            goals: Vec::new(),
            fears: Vec::new(),
            values: Vec::new(),
        }
    }
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            communication_style: None,
            work_style: None,
            response_format: None,
        }
    }
}
