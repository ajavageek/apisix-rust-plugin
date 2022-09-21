use log::warn;
use proxy_wasm::traits::{Context, HttpContext};
use proxy_wasm::types::{Action, LogLevel};

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_http_context(|_, _| -> Box<dyn HttpContext> { Box::new(HttpCall) });
}}

struct HttpCall;

impl Context for HttpCall {}

impl HttpContext for HttpCall {
    fn on_http_request_headers(&mut self, _: usize, _: bool) -> Action {
        warn!("on_http_request_headers");
        Action::Continue
    }
}
