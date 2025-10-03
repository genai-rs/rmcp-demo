use serde::Serialize;
use serde_json::json;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Setup trace context and record input parameters for a tool function.
/// Call this at the beginning of your tool function.
pub async fn trace_setup_input<T: Serialize>(args: &T) {
    // Try to get stored trace context and attach it
    if let Some(ctx) = crate::trace_store::get_current_trace_context().await {
        // Ignore potential failure if the span is already closed
        let _ = tracing::Span::current().set_parent(ctx);
    }

    // Record input parameters as span attribute
    let input_json = json!(args);
    tracing::Span::current().record("input", tracing::field::display(&input_json.to_string()));
}

/// Convenience function that combines all tracing setup for RMCP tools.
/// Returns the extracted args after setting up tracing.
///
/// Usage:
/// ```rust
/// let args = trace_rmcp_setup(params).await;
/// ```
pub async fn trace_rmcp_setup<T: for<'de> serde::Deserialize<'de> + Serialize>(
    params: rmcp::handler::server::wrapper::Parameters<T>,
) -> T {
    let rmcp::handler::server::wrapper::Parameters(args) = params;

    // Setup trace context and input
    trace_setup_input(&args).await;

    args
}

/// Convenience function for recording output and returning result.
///
/// Usage:
/// ```rust
/// trace_rmcp_result(json!(&weather))
/// ```
pub fn trace_rmcp_result<T: Serialize>(
    output_data: T,
) -> Result<rmcp::model::CallToolResult, rmcp::ErrorData> {
    let json_value = json!(&output_data);
    tracing::Span::current().record("output", tracing::field::display(&json_value.to_string()));
    Ok(rmcp::model::CallToolResult::structured(json_value))
}
