use async_trait::async_trait;

#[async_trait]
pub trait HandleMessage {}

// handlers
pub struct MessageHandlerBase {}

impl HandleMessage for MessageHandlerBase {}
