use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// A procedural macro that automatically captures input parameters and output values
/// as trace span attributes for rmcp tool functions.
///
/// This macro will:
/// 1. Extract all parameters from Parameters<T> and record them as "input"
/// 2. Capture the return value and record it as "output" before returning
/// 3. Attach the stored trace context if available
///
/// Usage:
/// ```rust
/// #[trace_io]
/// async fn get_weather(
///     &self,
///     _request_context: RequestContext<RoleServer>,
///     params: Parameters<GetWeatherArgs>,
/// ) -> Result<CallToolResult, McpError> {
///     // your implementation
/// }
/// ```
#[proc_macro_attribute]
pub fn trace_io(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);

    let vis = &input.vis;
    let sig = &input.sig;
    let fn_name = &sig.ident;
    let inputs = &sig.inputs;
    let output = &sig.output;
    let block = &input.block;
    let attrs = &input.attrs;
    let asyncness = &sig.asyncness;
    let generics = &sig.generics;
    let where_clause = &sig.generics.where_clause;

    // We'll wrap the entire function body
    let wrapped_body = quote! {
        // Extract Parameters at the beginning
        let Parameters(args) = params;

        // Try to get stored trace context and attach it
        let stored_context = crate::trace_store::get_current_trace_context().await;
        if let Some(ctx) = stored_context {
            tracing::Span::current().set_parent(ctx);
        }

        // Record input
        let input_json = serde_json::json!(&args);
        tracing::Span::current().record("input", tracing::field::display(&input_json.to_string()));

        // Execute the original function body and capture the result
        let execute_body = async move {
            #block
        };
        let result = execute_body.await;

        // Record output if successful
        if let Ok(ref call_result) = result {
            // We need to extract the JSON from CallToolResult
            // Since CallToolResult::structured() takes a serde_json::Value,
            // we should capture that value before creating CallToolResult
            // This is a bit tricky without modifying the original body
            // For now, we'll just record what we can access
            if let Some(content) = call_result.content.first() {
                if let Some(ref text) = content.text {
                    tracing::Span::current().record("output", tracing::field::display(&text));
                }
            }
        }

        result
    };

    let result = quote! {
        #(#attrs)*
        #[tracing::instrument(skip(self, _request_context, params), fields(
            input = tracing::field::Empty,
            output = tracing::field::Empty
        ))]
        #vis #asyncness fn #fn_name #generics(#inputs) #output #where_clause {
            #wrapped_body
        }
    };

    TokenStream::from(result)
}
