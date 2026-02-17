use crate::aieos::types::*;

pub struct AIEOSPromptGenerator;

impl AIEOSPromptGenerator {
    pub fn generate_system_prompt(aieos: &AIEOS) -> String {
        let mut sections = Vec::new();

        sections.push("# Identity".to_string());
        sections.push(Self::generate_identity(&aieos.identity));
        
        sections.push("\n# Psychology".to_string());
        sections.push(Self::generate_psychology(&aieos.psychology));
        
        sections.push("\n# Communication Style".to_string());
        sections.push(Self::generate_linguistics(&aieos.linguistics));
        
        sections.push("\n# Motivations".to_string());
        sections.push(Self::generate_motivations(&aieos.motivations));

        if !aieos.capabilities.is_empty() {
            sections.push("\n# Capabilities".to_string());
            sections.push(Self::generate_capabilities(&aieos.capabilities));
        }

        if let Some(prefs) = Self::generate_preferences(&aieos.preferences) {
            sections.push("\n# Preferences".to_string());
            sections.push(prefs);
        }

        sections.join("\n\n")
    }

    pub fn generate_identity(identity: &Identity) -> String {
        let mut parts = vec![
            format!("- Name: {}", identity.names.first),
        ];
        
        if let Some(nickname) = &identity.names.nickname {
            parts.push(format!("- Nickname: {}", nickname));
        }
        if let Some(middle) = &identity.names.middle {
            parts.push(format!("- Middle Name: {}", middle));
        }
        if let Some(last) = &identity.names.last {
            parts.push(format!("- Last Name: {}", last));
        }
        if let Some(role) = &identity.role {
            parts.push(format!("- Role: {}", role));
        }
        if let Some(pronouns) = &identity.pronouns {
            parts.push(format!("- Pronouns: {}/{}", pronouns.subject, pronouns.object));
        }
        if let Some(avatar) = &identity.avatar {
            parts.push(format!("- Avatar: {}", avatar));
        }
        
        parts.join("\n")
    }

    pub fn generate_psychology(psychology: &Psychology) -> String {
        let mut parts = Vec::new();

        parts.push("## Neural Matrix".to_string());
        let nm = &psychology.neural_matrix;
        parts.push(format!("- Creativity: {:.0}%", nm.creativity * 100.0));
        parts.push(format!("- Logic: {:.0}%", nm.logic * 100.0));
        parts.push(format!("- Empathy: {:.0}%", nm.empathy * 100.0));
        parts.push(format!("- Courage: {:.0}%", nm.courage * 100.0));
        parts.push(format!("- Patience: {:.0}%", nm.patience * 100.0));

        if let Some(mbti) = &psychology.traits.mbti {
            parts.push("\n## Personality Type".to_string());
            parts.push(format!("- MBTI: {}", mbti));
        }

        if let Some(big_five) = &psychology.traits.big_five {
            parts.push("\n## Big Five Traits".to_string());
            parts.push(format!("- Openness: {:.0}%", big_five.openness * 100.0));
            parts.push(format!("- Conscientiousness: {:.0}%", big_five.conscientiousness * 100.0));
            parts.push(format!("- Extraversion: {:.0}%", big_five.extraversion * 100.0));
            parts.push(format!("- Agreeableness: {:.0}%", big_five.agreeableness * 100.0));
            parts.push(format!("- Neuroticism: {:.0}%", big_five.neuroticism * 100.0));
        }

        if !psychology.traits.custom.is_empty() {
            parts.push("\n## Custom Traits".to_string());
            for trait_name in &psychology.traits.custom {
                parts.push(format!("- {}", trait_name));
            }
        }

        if let Some(compass) = &psychology.moral_compass {
            parts.push("\n## Moral Compass".to_string());
            parts.push(format!("- Alignment: {}", compass.alignment));
            if let Some(framework) = &compass.ethics_framework {
                parts.push(format!("- Ethics Framework: {}", framework));
            }
        }

        if let Some(emotional) = &psychology.emotional_model {
            parts.push("\n## Emotional Model".to_string());
            parts.push(format!("- Volatility: {:.0}%", emotional.volatility * 100.0));
            parts.push(format!("- Warmth: {:.0}%", emotional.warmth * 100.0));
            if !emotional.mood_patterns.is_empty() {
                parts.push("- Mood Patterns:".to_string());
                for pattern in &emotional.mood_patterns {
                    parts.push(format!("  - {}", pattern));
                }
            }
        }

        parts.join("\n")
    }

    pub fn generate_linguistics(linguistics: &Linguistics) -> String {
        let style = &linguistics.text_style;
        let mut parts = Vec::new();

        parts.push(format!("- Formality: {:.0}%", style.formality_level * 100.0));
        parts.push(format!("- Uses Slang: {}", if style.slang_usage { "Yes" } else { "No" }));

        if let Some(humor) = style.humor_level {
            parts.push(format!("- Humor Level: {:.0}%", humor * 100.0));
        }

        if let Some(technical) = style.technical_level {
            parts.push(format!("- Technical Level: {:.0}%", technical * 100.0));
        }

        if let Some(length) = &style.sentence_length {
            parts.push(format!("- Sentence Length: {}", length));
        }

        if let Some(tone) = &style.tone {
            parts.push(format!("- Tone: {}", tone));
        }

        if !linguistics.speech_patterns.is_empty() {
            parts.push("\n## Speech Patterns".to_string());
            for pattern in &linguistics.speech_patterns {
                let freq = pattern.frequency.map(|f| format!(" ({:.0}%)", f * 100.0)).unwrap_or_default();
                parts.push(format!("- \"{}\"{}", pattern.pattern, freq));
            }
        }

        if let Some(vocab) = &linguistics.vocabulary {
            if !vocab.technical_terms.is_empty() || !vocab.preferred_words.is_empty() {
                parts.push("\n## Vocabulary".to_string());
                
                if !vocab.technical_terms.is_empty() {
                    parts.push("- Technical Terms:".to_string());
                    for term in &vocab.technical_terms {
                        parts.push(format!("  - {}", term));
                    }
                }
                
                if !vocab.preferred_words.is_empty() {
                    parts.push("- Preferred Words:".to_string());
                    for word in &vocab.preferred_words {
                        parts.push(format!("  - {}", word));
                    }
                }
                
                if !vocab.avoided_words.is_empty() {
                    parts.push("- Avoided Words:".to_string());
                    for word in &vocab.avoided_words {
                        parts.push(format!("  - {}", word));
                    }
                }
            }
        }

        parts.join("\n")
    }

    pub fn generate_motivations(motivations: &Motivations) -> String {
        let mut parts = vec![format!("- Core Drive: {}", motivations.core_drive)];
        
        if !motivations.goals.is_empty() {
            parts.push("\n## Goals".to_string());
            for goal in &motivations.goals {
                parts.push(format!("- {}", goal));
            }
        }

        if !motivations.fears.is_empty() {
            parts.push("\n## Fears".to_string());
            for fear in &motivations.fears {
                parts.push(format!("- {}", fear));
            }
        }

        if !motivations.values.is_empty() {
            parts.push("\n## Values".to_string());
            for value in &motivations.values {
                parts.push(format!("- {}", value));
            }
        }

        parts.join("\n")
    }

    pub fn generate_capabilities(capabilities: &[Capability]) -> String {
        let mut parts = Vec::new();

        if !capabilities.is_empty() {
            for cap in capabilities {
                let mut lines = vec![format!("## {}", cap.name)];
                lines.push(format!("- Level: {}", cap.level));
                if let Some(desc) = &cap.description {
                    lines.push(format!("- Description: {}", desc));
                }
                parts.push(lines.join("\n"));
            }
        }

        parts.join("\n\n")
    }

    pub fn generate_preferences(preferences: &Preferences) -> Option<String> {
        let mut parts = Vec::new();

        if let Some(style) = &preferences.communication_style {
            parts.push(format!("- Communication Style: {}", style));
        }

        if let Some(style) = &preferences.work_style {
            parts.push(format!("- Work Style: {}", style));
        }

        if let Some(format) = &preferences.response_format {
            parts.push(format!("- Response Format: {}", format));
        }

        if parts.is_empty() {
            None
        } else {
            Some(parts.join("\n"))
        }
    }

    pub fn generate_compact(aieos: &AIEOS) -> String {
        let name = &aieos.identity.names.first;
        let _nickname = aieos.identity.names.nickname.as_deref().unwrap_or(name);
        
        let mut prompt = format!("You are {}.", name);
        
        if let Some(role) = &aieos.identity.role {
            prompt.push_str(&format!(" You are a {}.", role));
        }
        
        if let Some(mbti) = &aieos.psychology.traits.mbti {
            prompt.push_str(&format!(" Your MBTI personality type is {}.", mbti));
        }
        
        prompt.push_str(&format!(" Your core drive is: {}.", aieos.motivations.core_drive));
        
        let nm = &aieos.psychology.neural_matrix;
        if nm.creativity > 0.7 {
            prompt.push_str(" You are highly creative.");
        }
        if nm.empathy > 0.7 {
            prompt.push_str(" You are very empathetic.");
        }
        
        let style = &aieos.linguistics.text_style;
        if style.formality_level > 0.7 {
            prompt.push_str(" You communicate in a formal manner.");
        } else if style.formality_level < 0.3 {
            prompt.push_str(" You communicate in a casual, friendly manner.");
        }

        prompt
    }
}
