use regex::Regex;

use chatbot::handler::{MessageHandler,HandlerResult};
use chatbot::message::IncomingMessage;

// Silly debug handler

pub struct DebugHandler {
    re: Regex,
}

impl DebugHandler {
    pub fn new() -> DebugHandler {
        DebugHandler {
            re: Regex::new(r".").unwrap()
        }
    }
}

impl MessageHandler for DebugHandler {
    fn name(&self) -> &str {
        "DebugHandler"
    }

    fn re(&self) -> &Regex {
        &self.re
    }

    fn handle(&self, incoming: &IncomingMessage) -> HandlerResult {
        println!("<- {:?}", incoming);
        Ok(())
    }
}
