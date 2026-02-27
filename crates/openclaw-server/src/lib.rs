//! OpenClaw Server - HTTP/WebSocket 服务

pub mod adapters;
pub mod acp_service;
pub mod agent_service;
pub mod agentic_rag;
pub mod agentic_rag_api;
pub mod api;
pub mod app_context;
pub mod browser_api;
pub mod canvas_api;
pub mod channel_message_handler;
pub mod channel_service;
pub mod config_adapter;
pub mod server_config;
pub mod device_api;
pub mod device_manager;
pub mod gateway;
pub mod gateway_service;
pub mod hardware_tools;
pub mod orchestrator;
pub mod ports;
pub mod service_factory;
pub mod sse;
pub mod vector_store_registry;
pub mod voice_service;
pub mod websocket;

pub use gateway_service::Gateway;
pub use app_context::AppContext;
pub use server_config::ServerConfig;
pub use service_factory::{ServiceFactory, DefaultServiceFactory};
pub use acp_service::{AcpService, RouterResult};
