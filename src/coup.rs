use regex::Regex;

use std::sync::{Arc, Mutex};
use chatbot::Chatbot;
use chatbot::handler::{MessageHandler,HandlerResult};
use chatbot::message::IncomingMessage;

macro_rules! game{
    ($s:ident) => {
        $s.game.lock().unwrap()
    }
}

pub enum Role {
    Ambassador,
    Assassin,
    Captain,
    Contessa,
    Duke,
}

pub enum Card {
    Alive(Role),
    Dead(Role),
}

pub struct Player {
    c1: Role,
    c2: Role,
    coins: u8,
}

type WrappedGame = Arc<Mutex<Game>>;

// Storing the players in a vec and treating it like a circular buffer simplifies bookkeeping and
// given that the game is capped at 6 players the linear search isn't so bad.
pub struct Game {
    players: Vec<Player>,
    started: bool,
    turn: u8,
}

impl Game {
    pub fn new() -> Game {
        Game {
            players: vec![],
            started: false,
            turn: 0,
        }
    }

    /// Binds this game to the chatbot, creating handlers for everything required.
    pub fn bind(self, bot: &mut Chatbot) {
        let wrapped = Arc::new(Mutex::new(self));
        let start = StartHandler::new(wrapped);
        bot.add_handler(start);
    }
}

pub struct StartHandler {
    re: Regex,
    game: WrappedGame,
}

impl StartHandler {
    fn new(game: WrappedGame) -> StartHandler {
        StartHandler {
            re: Regex::new(r"!start").unwrap(),
            game: game,
        }
    }

    fn start(&self, incoming: &IncomingMessage) {
        let mut game = game!(self);
        if game.started {
            incoming.reply("Game already started".to_string());
        } else if game.players.len() < 2 {
            incoming.reply("Need more than 2 players".to_string());
        } else {
            game.started = true;
            incoming.reply("Starting the game!".to_string());
        }
    }
}

impl MessageHandler for StartHandler {
    fn name(&self) -> &str {
        "StartHandler"
    }

    fn re(&self) -> &Regex {
        &self.re
    }

    fn handle(&self, incoming: &IncomingMessage) -> HandlerResult {
        self.start(incoming);
        Ok(())
    }
}

