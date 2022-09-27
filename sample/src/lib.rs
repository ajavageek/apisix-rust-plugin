use log::{error, warn};
use proxy_wasm::traits::{Context, HttpContext, RootContext};
use proxy_wasm::types::{Action, ContextType, LogLevel};
use serde::Deserialize;
use serde_json::{Map, Value};

proxy_wasm::main! {{
    proxy_wasm::set_log_level(LogLevel::Trace);
    proxy_wasm::set_root_context(|_| -> Box<dyn RootContext> { Box::new(new_root()) });
}}

#[derive(Deserialize, Clone)]
struct Headers {
    add: Option<Map<String, Value>>,
    set: Option<Map<String, Value>>,
}

#[derive(Deserialize, Clone)]
struct Config {
    headers: Headers,
    body: String,
}

struct HttpCall {
    config: Config,
}

impl Context for HttpCall {}

fn get_nginx_variable_if_possible(ctx: &HttpCall, value: &Value) -> String {
    let value = value.as_str().unwrap();
    if value.starts_with('$') {
        let option = ctx.get_property(vec![&value[1..value.len()]])
            .and_then(|bytes| String::from_utf8(bytes).ok());
        return if let Some(nginx_value) = option {
            nginx_value
        } else {
            value.to_string()
        }
    }
    value.to_string()
}

impl HttpContext for HttpCall {
    fn on_http_response_headers(&mut self, _num_headers: usize, end_of_stream: bool) -> Action {
        warn!("on_http_response_headers");
        if end_of_stream {
            if self.config.headers.add.is_some() {
                let add_headers = self.config.headers.add.as_ref().unwrap();
                for (key, value) in add_headers.into_iter() {
                    let value = get_nginx_variable_if_possible(self, value);
                    self.add_http_response_header(key, value.as_str());
                }
            }
            if self.config.headers.set.is_some() {
                let set_headers = self.config.headers.set.as_ref().unwrap();
                for (key, value) in set_headers.into_iter() {
                    let value = get_nginx_variable_if_possible(self, value);
                    self.set_http_response_header(key, Some(&value));
                }
            }
            let body = &self.config.body;
            if !body.is_empty() {
                warn!("Rewrite body is configured, letting Nginx know about it");
                self.set_property(vec!["wasm_process_resp_body"], Some("true".as_bytes()));
                warn!("Rewrite body is configured, resetting Content-Length");
                self.set_http_response_header("Content-Length", None)
            }
        }
        Action::Continue
    }

    fn on_http_response_body(&mut self, _body_size: usize, end_of_stream: bool) -> Action {
        warn!("on_http_response_body");
        let body = &self.config.body;
        if !body.is_empty() {
            if end_of_stream {
                warn!("Rewrite body is configured, rewriting {}", body);
                let body = self.config.body.as_bytes();
                self.set_http_response_body(0, body.len(),body);
            } else {
                return Action::Pause;
            }
        }
        Action::Continue
    }
}

struct HttpCallRoot {
    config: Config,
}

impl Context for HttpCallRoot {}

fn new_root() -> HttpCallRoot {
    HttpCallRoot { config: Config { headers: Headers { add: None, set: None }, body: "".to_string() } }
}

impl RootContext for HttpCallRoot {
    fn on_configure(&mut self, _: usize) -> bool {
        if let Some(config_bytes) = self.get_plugin_configuration() {
            let result = String::from_utf8(config_bytes)
                .map_err(|e| e.utf8_error().to_string())
                .and_then(|s| serde_json::from_str(&s).map_err(|e| e.to_string()));
            return match result {
                Ok(config) => {
                    self.config = config;
                    true
                }
                Err(message) => {
                    error!("An error occurred while reading the configuration file: {}", message);
                    false
                }
            };
        }
        true
    }

    fn create_http_context(&self, _context_id: u32) -> Option<Box<dyn HttpContext>> {
        Some(Box::new(HttpCall {
            config: self.config.clone(),
        }))
    }

    fn get_type(&self) -> Option<ContextType> {
        Some(ContextType::HttpContext)
    }
}
