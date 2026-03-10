// Shared utilities for SafePaw

use serde::{Deserialize, Serialize};
use serde_json::Value;

// ============================================================================
// Handler Result Type - Used by CLI and REST API handlers
// ============================================================================

/// Result type for handlers - contains success message or error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerResult<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_details: Option<Value>,
}

impl<T> HandlerResult<T> {
    pub fn ok(data: T, message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: message.into(),
            error_details: None,
        }
    }

    pub fn ok_with_message(message: impl Into<String>) -> Self {
        Self {
            success: true,
            data: None,
            message: message.into(),
            error_details: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            message: message.into(),
            error_details: None,
        }
    }

    pub fn err_with_details(message: impl Into<String>, error_details: Value) -> Self {
        Self {
            success: false,
            data: None,
            message: message.into(),
            error_details: Some(error_details),
        }
    }
}
