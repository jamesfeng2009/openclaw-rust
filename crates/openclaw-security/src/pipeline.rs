use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

use crate::audit::{AuditEvent, AuditEventType, AuditLogger, AuditSeverity};
use crate::classifier::{PromptCategory, PromptClassifier, LlmClassification};
use crate::input_filter::InputFilter;
use crate::self_healer::{SelfHealer, OperationState, RecoveryStrategy};
use crate::validator::{OutputValidator, OutputValidation};

#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub enable_input_filter: bool,
    pub enable_classifier: bool,
    pub enable_output_validation: bool,
    pub enable_audit: bool,
    pub enable_self_healer: bool,
    pub classifier_strict_mode: bool,
    pub stuck_timeout: Duration,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            enable_input_filter: true,
            enable_classifier: true,
            enable_output_validation: true,
            enable_audit: true,
            enable_self_healer: true,
            classifier_strict_mode: false,
            stuck_timeout: Duration::from_secs(30),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PipelineResult {
    Allow,
    Block(String),
    Warn(String),
}

pub struct SecurityPipeline {
    config: PipelineConfig,
    input_filter: InputFilter,
    classifier: PromptClassifier,
    output_validator: OutputValidator,
    audit_logger: Arc<AuditLogger>,
    self_healer: Arc<SelfHealer>,
}

impl Default for SecurityPipeline {
    fn default() -> Self {
        Self::new(PipelineConfig::default())
    }
}

impl SecurityPipeline {
    pub fn new(config: PipelineConfig) -> Self {
        let audit_logger = Arc::new(AuditLogger::new());
        let self_healer = Arc::new(
            SelfHealer::new()
                .with_timeout(config.stuck_timeout)
        );

        Self {
            config,
            input_filter: InputFilter::new(),
            classifier: PromptClassifier::new(),
            output_validator: OutputValidator::new(),
            audit_logger,
            self_healer,
        }
    }

    pub fn get_audit_logger(&self) -> Arc<AuditLogger> {
        self.audit_logger.clone()
    }

    pub fn get_self_healer(&self) -> Arc<SelfHealer> {
        self.self_healer.clone()
    }

    pub async fn check_input(&self, session_id: &str, input: &str) -> (PipelineResult, Option<LlmClassification>) {
        if self.config.enable_audit {
            self.audit_logger.log_input(session_id, input).await;
        }

        if self.config.enable_input_filter {
            let filtered = self.input_filter.check(input).await;
            if !filtered.allowed {
                if self.config.enable_audit {
                    self.audit_logger.log_filtered(session_id, input, &filtered.reason).await;
                }
                return (PipelineResult::Block(filtered.reason), None);
            }
        }

        if self.config.enable_classifier {
            let context = if self.config.classifier_strict_mode {
                Some("strict_mode")
            } else {
                None
            };
            
            let classification = self.classifier.classify(input, context).await;
            
            if self.config.enable_audit {
                self.audit_logger.log_classification(
                    session_id,
                    &classification.category,
                    classification.risk_score
                ).await;
            }

            let result = match classification.category {
                PromptCategory::Critical => {
                    PipelineResult::Block("Critical threat detected".to_string())
                }
                PromptCategory::Malicious => {
                    PipelineResult::Block("Malicious prompt detected".to_string())
                }
                PromptCategory::Suspicious => {
                    if self.config.classifier_strict_mode {
                        PipelineResult::Block("Suspicious prompt in strict mode".to_string())
                    } else {
                        PipelineResult::Warn("Suspicious prompt detected".to_string())
                    }
                }
                PromptCategory::Benign | PromptCategory::Safe => {
                    PipelineResult::Allow
                }
            };

            (result, Some(classification))
        } else {
            (PipelineResult::Allow, None)
        }
    }

    pub async fn validate_output(&self, session_id: &str, output: &str) -> (String, OutputValidation) {
        if !self.config.enable_output_validation {
            return (output.to_string(), OutputValidation {
                level: crate::validator::ValidationLevel::Safe,
                matches: vec![],
                total_count: 0,
                requires_action: false,
            });
        }

        let (redacted, validation) = self.output_validator.validate_and_redact(output).await;

        if self.config.enable_audit {
            self.audit_logger.log_validation(
                session_id,
                validation.total_count,
                validation.requires_action
            ).await;
        }

        (redacted, validation)
    }

    pub async fn start_operation(&self, session_id: &str, tool_id: &str, action: &str) -> String {
        if self.config.enable_self_healer {
            let op_id = self.self_healer.start_operation(tool_id, action).await;
            
            if self.config.enable_audit {
                self.audit_logger.log_tool_execution(
                    session_id,
                    tool_id,
                    action,
                    "",
                    "started",
                    0
                ).await;
            }
            
            op_id
        } else {
            String::new()
        }
    }

    pub async fn record_progress(&self, operation_id: &str) {
        if self.config.enable_self_healer {
            self.self_healer.record_progress(operation_id).await;
        }
    }

    pub async fn complete_operation(&self, session_id: &str, operation_id: &str, result: &str, duration_ms: u64) {
        if self.config.enable_self_healer {
            self.self_healer.complete_operation(operation_id).await;

            if self.config.enable_audit {
                self.audit_logger.log_tool_execution(
                    session_id,
                    "",
                    "",
                    "",
                    result,
                    duration_ms
                ).await;
            }
        }
    }

    pub async fn check_stuck_operations(&self) -> Vec<String> {
        if !self.config.enable_self_healer {
            return vec![];
        }

        let stuck = self.self_healer.check_for_stuck_operations().await;
        stuck.iter().map(|s| s.operation_id.clone()).collect()
    }

    pub async fn attempt_recovery(&self, operation_id: &str, strategy: RecoveryStrategy) -> bool {
        if !self.config.enable_self_healer {
            return false;
        }

        self.self_healer.attempt_recovery(operation_id, strategy).await
    }

    pub async fn get_stats(&self) -> PipelineStats {
        PipelineStats {
            audit_stats: if self.config.enable_audit {
                Some(self.audit_logger.get_stats().await)
            } else {
                None
            },
            self_healer_stats: if self.config.enable_self_healer {
                Some(self.self_healer.get_recovery_stats().await)
            } else {
                None
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct PipelineStats {
    pub audit_stats: Option<std::collections::HashMap<AuditEventType, u32>>,
    pub self_healer_stats: Option<std::collections::HashMap<String, u32>>,
}
