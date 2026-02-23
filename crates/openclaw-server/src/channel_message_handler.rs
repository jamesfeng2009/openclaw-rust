use std::sync::Arc;
use async_trait::async_trait;
use openclaw_channels::{ChannelEvent, ChannelHandler, SendMessage};
use openclaw_core::{Result, OpenClawError};

pub struct ChannelMessageHandler {
    processor: Arc<dyn ChannelMessageProcessor>,
}

#[async_trait]
impl ChannelHandler for ChannelMessageHandler {
    async fn handle(
        &self,
        message: openclaw_channels::ChannelMessage,
    ) -> Result<Option<SendMessage>> {
        let channel_name = message.chat_id.clone();
        let content = message.content.clone();

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
    Arc::new(ChannelMessageHandler { processor })
}
