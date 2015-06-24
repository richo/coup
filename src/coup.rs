use regex::Regex;
use rand;
use rand::{Rng};
use std::sync::{Arc, Mutex};
use chatbot::Chatbot;
use chatbot::handler::{MessageHandler,HandlerResult};
use chatbot::message::IncomingMessage;

macro_rules! game{
    ($s:ident) => {
        $s.game.lock().unwrap()
    }
}

pub struct Deck {
    cards: Vec<Role>,
}

impl Deck {
    pub fn new() -> Deck {
        let cards = vec![
            Role::Ambassador, Role::Ambassador,
            Role::Assassin, Role::Assassin,
            Role::Captain, Role::Captain,
            Role::Contessa, Role::Contessa,
            Role::Duke, Role::Duke,
        ];
        Deck {
            cards: cards,
        }
    }

    pub fn take(&mut self) -> Role {
        self.cards.pop().unwrap()
    }

    pub fn shuffle(&mut self) {
        rand::thread_rng().shuffle(&mut self.cards);
    }
}

#[derive(Debug)]
pub enum Role {
    Ambassador,
    Assassin,
    Captain,
    Contessa,
    Duke,
}

#[derive(Debug)]
pub enum Card {
    Alive(Role),
    Dead(Role),
}

#[derive(Debug)]
pub struct Player {
    c1: Card,
    c2: Card,
    coins: u8,
    nick: String,
}

type WrappedGame = Arc<Mutex<Game>>;

// Storing the players in a vec and treating it like a circular buffer simplifies bookkeeping and
// given that the game is capped at 6 players the linear search isn't so bad.
pub struct Game {
    players: Vec<Player>,
    started: bool,
    deck: Deck,
    turn: u8,
}

impl Game {
    pub fn new() -> Game {
        let mut deck = Deck::new();
        deck.shuffle();
        Game {
            players: vec![],
            started: false,
            deck: deck,
            turn: 0,
        }
    }

    pub fn find_player(&self, nick: &str) -> Option<&Player> {
        for p in &self.players {
            if p.nick == nick {
                return Some(&p)
            }
        }
        None
    }

    /// Binds this game to the chatbot, creating handlers for everything required.
    pub fn bind(self, bot: &mut Chatbot) {
        let wrapped = Arc::new(Mutex::new(self));
        bot.add_handler(StartHandler::new(wrapped.clone()));
        bot.add_handler(JoinHandler::new(wrapped.clone()));
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

pub struct JoinHandler {
    re: Regex,
    game: WrappedGame,
}

impl JoinHandler {
    fn new(game: WrappedGame) -> JoinHandler {
        JoinHandler {
            re: Regex::new(r"!join").unwrap(),
            game: game,
        }
    }

    fn join(&self, incoming: &IncomingMessage) {
        let mut game = game!(self);
        // TODO(richo) Check that this player isn't already in the game
        if game.started {
            incoming.reply("Game already started".to_string());
        } else if game.players.len() > 6 {
            incoming.reply("Can't have a game with more than 6 players".to_string());
        } else {
            let nick = incoming.user().unwrap().to_string();
            incoming.reply(format!("Welcome to the game, {}", &nick));
            let (c1, c2) = (game.deck.take(), game.deck.take());
            incoming.reply_private(format!("You hold a {:?} and a {:?}", c1, c2));
            // We just deal to players as they join
            let player = Player {
                c1: Card::Alive(c1),
                c2: Card::Alive(c2),
                coins: 2,
                nick: nick,
            };
            println!("[-] Dealt in: {:?}", player);
            game.players.push(player);
        }
    }
}

impl MessageHandler for JoinHandler {
    fn name(&self) -> &str {
        "JoinHandler"
    }

    fn re(&self) -> &Regex {
        &self.re
    }

    fn handle(&self, incoming: &IncomingMessage) -> HandlerResult {
        self.join(incoming);
        Ok(())
    }
}
