pub mod conversation;
pub mod gateway;
pub mod ledger;
pub mod output;
pub mod runtime;

pub use conversation::ConversationController;
pub use gateway::{GatewayInput, GatewaySource, InstanceIoGateway, TransportEnvelope};
pub use ledger::{EventCursor, LedgerEvent, LedgerEventKind, LedgerRole, MessageLedger};
pub use output::{
    DeliveryIdentity, OutputEvent, OutputRouter, OutputTarget, RoutedOutputs, dedupe_outputs,
};
pub use runtime::{
    ActiveTurn, ConversationStatus, FollowSubscription, IngressResult, InstanceRuntime,
    InterruptDecision, InterruptMode, QueuedMessage, RuntimeStatus, RuntimeSummary, TurnId,
    TurnState, TurnStatus,
};

pub fn default_runtime() -> InstanceRuntime {
    InstanceRuntime::new("global-main", "conv_main")
}
