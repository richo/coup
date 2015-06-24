extern crate rustc_serialize;
#[macro_use(handler)]
extern crate chatbot;
#[macro_use]
extern crate log;
extern crate regex;

mod debug;
mod config;

use chatbot::Chatbot;
use chatbot::adapter::IrcAdapter;

fn main() {
    let mut bot = Chatbot::new("coup");

    bot.add_handler(debug::DebugHandler::new());

    let cfg = config::CoupIrcConfig::from_file("coup-irc.json").unwrap();
    bot.add_adapter(IrcAdapter::new(cfg.into()));

    bot.run();
}
