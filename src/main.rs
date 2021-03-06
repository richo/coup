extern crate rustc_serialize;
#[macro_use(handler)]
extern crate chatbot;
#[macro_use]
extern crate log;
extern crate rand;
extern crate regex;

mod debug;
mod config;
mod coup;

use chatbot::Chatbot;
use chatbot::adapter::IrcAdapter;

fn main() {
    let mut bot = Chatbot::new("coup");

    bot.add_handler(debug::DebugHandler::new());

    let game = coup::Game::new();
    game.bind(&mut bot);

    let cfg = config::CoupIrcConfig::from_file("coup-irc.json").unwrap();
    bot.add_adapter(IrcAdapter::new(cfg.into()));

    bot.run();
}
