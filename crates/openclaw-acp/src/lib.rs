//! OpenClaw ACP (Agent Collaboration Protocol)
//!
//! ACP is a protocol for multi-agent collaboration in group chats.
//! It provides message routing, context sharing, and capability management.

pub mod capability;
pub mod context;
pub mod envelope;
pub mod error;
pub mod gene;
pub mod message;
pub mod registry;
pub mod router;
pub mod transport;

pub use capability::{Capability, CapabilityRegistry};
pub use context::{ContextManager, SharedContext};
pub use envelope::{AcpEnvelope, EnvelopeType};
pub use error::AcpError;
pub use gene::{Gene, GeneCapsule};
pub use message::{AcpRequest, AcpResponse, AcpEvent};
pub use registry::{AgentInfo, AgentRegistry};
pub use router::Router;
pub use transport::Transport;
