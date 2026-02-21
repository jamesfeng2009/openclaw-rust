//! 应用上下文 - 持有所有服务实例

use std::sync::Arc;
use tokio::sync::RwLock;

use openclaw_ai::AIProvider;
use openclaw_core::Config;
use openclaw_device::UnifiedDeviceManager;
use openclaw_memory::MemoryManager;
use openclaw_security::pipeline::SecurityPipeline;
use openclaw_tools::ToolRegistry;

use crate::device_manager::DeviceManager;
use crate::orchestrator::ServiceOrchestrator;
use crate::voice_service::VoiceService;
use crate::vector_store_registry::VectorStoreRegistry;

#[derive(Clone)]
pub struct AppContext {
    pub config: Config,
    pub ai_provider: Arc<dyn AIProvider>,
    pub memory_manager: Option<Arc<MemoryManager>>,
    pub security_pipeline: Arc<SecurityPipeline>,
    pub tool_registry: Arc<ToolRegistry>,
    pub orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
    pub device_manager: Arc<DeviceManager>,
    pub unified_device_manager: Arc<UnifiedDeviceManager>,
    pub voice_service: Arc<VoiceService>,
    pub vector_store_registry: Arc<VectorStoreRegistry>,
}

impl AppContext {
    pub fn new(
        config: Config,
        ai_provider: Arc<dyn AIProvider>,
        memory_manager: Option<Arc<MemoryManager>>,
        security_pipeline: Arc<SecurityPipeline>,
        tool_registry: Arc<ToolRegistry>,
        orchestrator: Arc<RwLock<Option<ServiceOrchestrator>>>,
        device_manager: Arc<DeviceManager>,
        unified_device_manager: Arc<UnifiedDeviceManager>,
        voice_service: Arc<VoiceService>,
        vector_store_registry: Arc<VectorStoreRegistry>,
    ) -> Self {
        Self {
            config,
            ai_provider,
            memory_manager,
            security_pipeline,
            tool_registry,
            orchestrator,
            device_manager,
            unified_device_manager,
            voice_service,
            vector_store_registry,
        }
    }
}
