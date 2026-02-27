use std::sync::Arc;
use async_trait::async_trait;
use openclaw_channels::{ChannelEvent, ChannelHandler, SendMessage};
use openclaw_core::{Result, OpenClawError};

use crate::acp_service::AcpService;

pub struct ChannelMessageHandler {
    processor: Arc<dyn ChannelMessageProcessor>,
    acp_service: Option<Arc<AcpService>>,
}

impl ChannelMessageHandler {
    pub fn new(processor: Arc<dyn ChannelMessageProcessor>) -> Self {
        Self {
            processor,
            acp_service: None,
        }
    }

    pub fn with_acp_service(mut self, acp_service: Arc<AcpService>) -> Self {
        self.acp_service = Some(acp_service);
        self
    }

    fn parse_mentions(&self, content: &str) -> Vec<String> {
        let mut mentions = Vec::new();
        
        let feishu_pattern = regex::Regex::new(r"@([\w\u4e00-\u9fa5]+)").unwrap();
        for cap in feishu_pattern.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                mentions.push(name.as_str().to_string());
            }
        }

        let discord_pattern = regex::Regex::new(r"<@!?(\d+)>").unwrap();
        for _ in discord_pattern.captures_iter(content) {
            mentions.push("discord_mention".to_string());
        }

        mentions
    }

    fn clean_content(&self, content: &str) -> String {
        let mut cleaned = content.to_string();
        
        let feishu_pattern = regex::Regex::new(r"@[\w\u4e00-\u9fa5]+").unwrap();
        cleaned = feishu_pattern.replace_all(&cleaned, "").to_string();
        
        let discord_pattern = regex::Regex::new(r"<@!?\d+>").unwrap();
        cleaned = discord_pattern.replace_all(&cleaned, "").to_string();
        
        cleaned.trim().to_string()
    }
}

#[async_trait]
impl ChannelHandler for ChannelMessageHandler {
    async fn handle(
        &self,
        message: openclaw_channels::ChannelMessage,
    ) -> Result<Option<SendMessage>> {
        let channel_name = message.chat_id.clone();
        let content = message.content.clone();
        let user_id = message.user_id.clone();

        if let Some(acp) = &self.acp_service {
            let mentions = self.parse_mentions(&content);
            
            if !mentions.is_empty() {
                let result = acp.route_message(&content, &channel_name).await;
                
                if !result.target_agent.is_empty() {
                    let cleaned = self.clean_content(&content);
                    
                    match acp.handle_message(&cleaned, &channel_name, Some(&user_id)).await {
                        Ok(response) => {
                            return Ok(Some(SendMessage {
                                chat_id: channel_name,
                                message_type: "text".to_string(),
                                content: response,
                                title: None,
                                url: None,
                                at_mobiles: None,
                                mentioned_list: None,
                                base64: None,
                                md5: None,
                                articles: None,
                                media_id: None,
                            }));
                        }
                        Err(e) => {
                            tracing::warn!("ACP handle message failed: {:?}", e);
                        }
                    }
                }
            }
        }

        let response = self
            .processor
            .process_message(&channel_name, content)
            .await?;

        Ok(Some(SendMessage {
            chat_id: channel_name,
            message_type: "text".to_string(),
            content: response,
            title: None,
            url: None,
            at_mobiles: None,
            mentioned_list: None,
            base64: None,
            md5: None,
            articles: None,
            media_id: None,
        }))
    }
}

#[async_trait]
pub trait ChannelMessageProcessor: Send + Sync {
    async fn process_message(&self, channel_name: &str, message: String) -> Result<String>;
}

pub struct OrchestratorMessageProcessor {
    pub orchestrator: Arc<crate::orchestrator::ServiceOrchestrator>,
}

#[async_trait]
impl ChannelMessageProcessor for OrchestratorMessageProcessor {
    async fn process_message(&self, channel_name: &str, message: String) -> Result<String> {
        let response = self
            .orchestrator
            .process_channel_message(channel_name, message)
            .await?;
        Ok(response.content)
    }
}

pub fn create_channel_handler<P: ChannelMessageProcessor + 'static>(
    processor: Arc<P>,
) -> Arc<dyn ChannelHandler> {
    Arc::new(ChannelMessageHandler::new(processor))
}

pub fn create_channel_handler_with_acp<P: ChannelMessageProcessor + 'static>(
    processor: Arc<P>,
    acp_service: Arc<AcpService>,
) -> Arc<dyn ChannelHandler> {
    Arc::new(ChannelMessageHandler::new(processor).with_acp_service(acp_service))
}
