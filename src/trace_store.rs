use once_cell::sync::Lazy;
use opentelemetry::Context;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global storage for trace contexts indexed by session ID
pub static TRACE_STORE: Lazy<Arc<RwLock<HashMap<String, Context>>>> =
    Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

/// Global storage for the most recent trace context (fallback)
pub static CURRENT_TRACE: Lazy<Arc<RwLock<Option<Context>>>> =
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Store a trace context for a session
pub async fn store_trace_context(session_id: String, context: Context) {
    let mut store = TRACE_STORE.write().await;
    let sid = session_id.clone();
    store.insert(session_id, context.clone());

    // Also store as current trace (fallback)
    let mut current = CURRENT_TRACE.write().await;
    *current = Some(context);

    tracing::debug!("Stored trace context for session: {}", sid);
}

/// Retrieve a trace context for a session
#[allow(dead_code)]
pub async fn get_trace_context(session_id: &str) -> Option<Context> {
    let store = TRACE_STORE.read().await;
    let context = store.get(session_id).cloned();
    if context.is_some() {
        tracing::debug!("Retrieved trace context for session: {}", session_id);
    } else {
        tracing::debug!("No trace context found for session: {}", session_id);
    }
    context
}

/// Clear trace context for a session
#[allow(dead_code)]
pub async fn clear_trace_context(session_id: &str) {
    let mut store = TRACE_STORE.write().await;
    if store.remove(session_id).is_some() {
        tracing::debug!("Cleared trace context for session: {}", session_id);
    }
}

/// Get the current trace context (fallback when session ID is not available)
pub async fn get_current_trace_context() -> Option<Context> {
    let current = CURRENT_TRACE.read().await;
    current.clone()
}