pub mod conversation;
pub mod gateway;
pub mod ledger;
pub mod output;
pub mod runtime;

pub use conversation::ConversationController;
pub use gateway::{GatewayInput, GatewaySource, InstanceIoGateway, TransportEnvelope};
pub use ledger::{LedgerEvent, LedgerEventKind, LedgerRole, MessageLedger};
pub use output::{OutputEvent, OutputRouter, OutputTarget};
pub use runtime::{
    ConversationStatus, IngressResult, InstanceRuntime, InterruptMode, RuntimeStatus,
    RuntimeSummary, TurnState,
};

pub fn default_runtime() -> InstanceRuntime {
    InstanceRuntime::new("global-main", "conv_main")
}
