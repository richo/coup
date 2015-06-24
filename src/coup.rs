use regex::Regex;
use rand;
use rand::{Rng};
use std::thread;
use std::sync::{Arc, Mutex};
use chatbot::Chatbot;
use chatbot::handler::{MessageHandler,HandlerResult};
use chatbot::message::IncomingMessage;

macro_rules! game{
    ($s:ident) => {
        $s.game.lock().unwrap()
    }
}

const OBJECTION_TIMEOUT: u32 = 5000;

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
    action_submitted: bool,
    bullshit_called: bool,
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
            action_submitted: false,
            bullshit_called: false,
        }
    }

    pub fn current_turn(&self) -> &Player {
        &self.players[self.turn as usize]
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
        bot.add_handler(ActionHandler::new(wrapped.clone()));
        bot.add_handler(ReactionHandler::new(wrapped.clone()));
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
            incoming.reply(format!("ding {} it's your turn", game.current_turn().nick));
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
        let nick = incoming.user().unwrap().to_string();
        // TODO(richo) Check that this player isn't already in the game
        if game.started {
            incoming.reply("Game already started".to_string());
        } else if game.find_player(&nick).is_some() {
            incoming.reply(format!("You're already in the game, {}", nick));
        } else if game.players.len() > 6 {
            incoming.reply("Can't have a game with more than 6 players".to_string());
        } else {
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

pub struct ActionHandler {
    re: Regex,
    game: WrappedGame,
}

impl ActionHandler {
    fn new(game: WrappedGame) -> ActionHandler {
        ActionHandler {
            re: Regex::new(r"!(?P<action>duke|tax|income|steal|assassinate|ambassador|foreignaid) ?(?P<target>[a-zA-Z0-9_]+)?").unwrap(),
            game: game,
        }
    }

    fn duke(&self, player: Player, incoming: &IncomingMessage) {
        // Kinda lurky. We announce the intention of the player to do a thing, make sure we drop
        // the lock
    }
}

#[derive(Debug)]
enum Action {
    Duke,
}

impl MessageHandler for ActionHandler {
    fn name(&self) -> &str {
        "ActionHandler"
    }

    fn re(&self) -> &Regex {
        &self.re
    }

    fn handle(&self, incoming: &IncomingMessage) -> HandlerResult {
        let mut game = game!(self);
        let nick = incoming.user().unwrap().to_string();

        let captures = self.get_captures(incoming.get_contents()).unwrap();
        let action = captures.name("action");
        let target = captures.name("target");

        if !game.started {
            println!("Ignoring attempt to {:?} by {} - the game hasn't started", action, nick);
            return Ok(());
        }

        if game.current_turn().nick != nick {
            println!("Ignoring attempt to {:?} by {} - it's not their turn", action, nick);
            return Ok(());
        }

        if game.action_submitted {
            incoming.reply("You've already made your choice".to_string());
            return Ok(());
        }

        // Make sure that we've dropped game
        let todo = match (action, target) {
            (Some("duke"), None) => Action::Duke,
            (_, _) => {
                incoming.reply("Oops, I didn't understand that".to_string());
                return Ok(());
            }
        };

        game.action_submitted = true;

        incoming.reply(format!("Alright jerks, you have {}s to object to {}",
                               OBJECTION_TIMEOUT / 1000, nick));
        println!("{:?} is attempting to {:?}", incoming.user(), todo);

        // We drop our lock on the game to allow the counteraction handler to have a chance to run
        // it's course, but we hang onto a clone of it's containing Arc so we can find it again in
        // our closure
        drop(game);
        let wrapper = self.game.clone();

        thread::spawn(move || {
            thread::sleep_ms(OBJECTION_TIMEOUT);
            let game = wrapper.lock().unwrap();

            if game.bullshit_called {
                println!("Someone called bullshit");
            } else {
                println!("Noone called bullshit");
            }
        });

        Ok(())
    }
}

pub struct ReactionHandler {
    re: Regex,
    game: WrappedGame,
}

impl ReactionHandler {
    fn new(game: WrappedGame) -> ActionHandler {
        ActionHandler {
            re: Regex::new(r"!(?P<reaction>bullshit|captain|ambassador|contessa)").unwrap(),
            game: game,
        }
    }

}

impl MessageHandler for ReactionHandler {
    fn name(&self) -> &str {
        "ActionHandler"
    }

    fn re(&self) -> &Regex {
        &self.re
    }

    fn handle(&self, incoming: &IncomingMessage) -> HandlerResult {
        let mut game = game!(self);
        let nick = incoming.user().unwrap().to_string();

        let captures = self.get_captures(incoming.get_contents()).unwrap();
        let reaction = captures.name("reaction");

        if !game.started {
            println!("Ignoring attempt to {:?} by {} - the game hasn't started", reaction, nick);
            return Ok(());
        }

        if !game.action_submitted {
            incoming.reply("No action to object to".to_string());
            return Ok(());
        }

        match reaction {
            Some("!bullshit") => {
                game.bullshit_called = true;
            },
            _ => println!("Nfi what happened"),
        }

        Ok(())
    }
}
