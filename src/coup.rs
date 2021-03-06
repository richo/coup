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

const OBJECTION_TIMEOUT: u32 = 5_000;

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

#[derive(Debug,Clone)]
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

impl Player {
    pub fn adjust_coins(&mut self, adjustment: i8) {
        let new: i8 = self.coins as i8 + adjustment;
        if new < 0 {
            self.coins = 0;
        } else {
            self.coins = new as u8;
        }
    }
}

type WrappedGame = Arc<Mutex<Game>>;

#[derive(Clone)]
pub struct State {
    action: Option<Action>,
    // Lurky, but I suspect I don't want to dance with the borrow checker trying to actually stash
    // the Player object
    bullshit: Option<String>,
    counter: Option<(String, Role)>,
    counter_bullshit: Option<String>,
}

impl State {
    fn new() -> State {
        State {
            action: None,
            bullshit: None,
            counter: None,
            counter_bullshit: None,
        }
    }
}

// Storing the players in a vec and treating it like a circular buffer simplifies bookkeeping and
// given that the game is capped at 6 players the linear search isn't so bad.
pub struct Game {
    players: Vec<Player>,
    started: bool,
    deck: Deck,
    turn: u8,
    state: State
}

impl Game {
    // TODO Split this out into a StartedGame or something, to make some of these methods
    // uncallable, since they can panic.
    pub fn new() -> Game {
        let mut deck = Deck::new();
        deck.shuffle();
        Game {
            players: vec![],
            started: false,
            deck: deck,
            turn: 0,
            state: State::new(),
        }
    }

    pub fn current_player(&self) -> &Player {
        &self.players[self.turn as usize]
    }

    pub fn current_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.turn as usize]
    }

    pub fn find_player(&self, nick: &str) -> Option<&Player> {
        for p in &self.players {
            if p.nick == nick {
                return Some(&p)
            }
        }
        None
    }

    pub fn find_player_mut(&mut self, nick: &str) -> Option<&mut Player> {
        for p in self.players.iter_mut() {
            if p.nick == nick {
                return Some(p)
            }
        }
        None
    }

    pub fn current_player_ding(&self) -> String {
        format!("ding {} it's your turn", self.current_player().nick)
    }

    pub fn next_turn<F: Fn(String)>(&mut self, f: F) {
        let players = self.players.len();
        self.turn = (self.turn + 1) % players as u8;
        self.state = State::new();
        f(self.current_player_ding());
    }

    /// Binds this game to the chatbot, creating handlers for everything required.
    pub fn bind(self, bot: &mut Chatbot) {
        let wrapped = Arc::new(Mutex::new(self));
        bot.add_handler(StartHandler::new(wrapped.clone()));
        bot.add_handler(JoinHandler::new(wrapped.clone()));
        bot.add_handler(ActionHandler::new(wrapped.clone()));
        bot.add_handler(ReactionHandler::new(wrapped.clone()));
    }

    // We have to define all the mutative actions on the game itself, because we've entirely lost
    // access to the handlers by the time we can poke them

    pub fn duke<F: Fn(String)>(&mut self, f: F) {
        {
            let mut current = self.current_player_mut();
            current.adjust_coins(3);
            f(format!("{} Duke'd (now at {})", current.nick, current.coins));
        }
        self.next_turn(f);
    }

    pub fn tax<F: Fn(String)>(&mut self, f: F) {
        {
            let mut current = self.current_player_mut();
            current.adjust_coins(1);
            f(format!("{} Tax'd (now at {})", current.nick, current.coins));
        }
        self.next_turn(f);
    }

    pub fn steal<F: Fn(String)>(&mut self, target: &str, f: F) {
        {
            {
                let mut target_player = self.find_player_mut(&target[..]).unwrap();
                target_player.adjust_coins(-2);
                // let target_coins = target_player.coins.clone();
                drop(target_player);
            }

            let mut current = self.current_player_mut();
            current.adjust_coins(2);
            f(format!("{0} stole from {1} ({0}: {2}, {1}, {3})", current.nick, target, current.coins, current.coins));
        }
        self.next_turn(f);
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
            incoming.reply(game.current_player_ding());
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
}

#[derive(Debug,Clone)]
enum Action {
    Duke,
    Steal(String),
}

impl MessageHandler for ActionHandler {
    fn name(&self) -> &str {
        "ActionHandler"
    }

    fn re(&self) -> &Regex {
        &self.re
    }

    // The contract of the ActionHandler's runloop is that:
    // * It awaits a valid command from the current player, ignoring all others
    // * Once it recieves on, it allots a timeout to either call bullshit (by anyone) or to block
    //   (by an eligable player)
    // * Once this timeout occurs, if noone has objected or blocked:
    //   - It either executes the action and begins again with the next turn, or
    // * If someone has called bullshit it:
    //   - Prompts the player to flip a card,
    //     (And then either kills their card or deals a new one)
    // * If someone has blocked:
    //   - The blocked player may either !cede or !bullshit

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

        if game.current_player().nick != nick {
            println!("Ignoring attempt to {:?} by {} - it's not their turn", action, nick);
            return Ok(());
        }

        if game.state.action.is_some() {
            incoming.reply("You've already made your choice".to_string());
            return Ok(());
        }

        let todo = match (action, target) {
            (Some("duke"), None) => Action::Duke,
            // This special case might not belong, but since it's unblockable we just do it.
            (Some("tax"), None) => {
                game.tax(|msg| {
                    incoming.reply(msg);
                });
                return Ok(());
            },
            (Some("steal"), Some(from)) => {
                if game.find_player(from).is_none() {
                    incoming.reply(format!("I don't know who {} is", from));
                    return Ok(());
                }

                Action::Steal(from.to_string())
            }
            (_, _) => {
                incoming.reply("Oops, I didn't understand that".to_string());
                return Ok(());
            }
        };

        println!("{:?} is attempting to {:?}", incoming.user(), todo);
        game.state.action = Some(todo);

        incoming.reply(format!("Alright jerks, you have {}s to object to {}",
                               OBJECTION_TIMEOUT / 1000, nick));

        // We drop our lock on the game to allow the counteraction handler to have a chance to run
        // it's course, but we hang onto a clone of it's containing Arc so we can find it again in
        // our closure
        drop(game);
        // TODO(richo) It seems like I really should add explicit functionality for this to chatbot
        let replypipe = incoming.clone();
        let wrapper = self.game.clone();

        thread::spawn(move || {
            thread::sleep_ms(OBJECTION_TIMEOUT);
            let mut game = wrapper.lock().unwrap();

            // This closure gets to actually Do A Thing iff noone objects

            if let State { action: Some(ref action),
                           bullshit: None,
                           counter: None,
                           counter_bullshit: None } = game.state.clone() {
               match action.clone() {
                   Action::Duke => {
                       game.duke(|msg| {
                           replypipe.reply(msg);
                       });
                   },
                   Action::Steal(ref target) => {
                       game.steal(&target[..], |msg| {
                           replypipe.reply(msg);
                       });
                   },
               }
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
            re: Regex::new(r"!(?P<reaction>bullshit|block duke|block captain|block ambassador|block contessa)").unwrap(),
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

    /// This handler is responsible for basically everything outside of the normal "I said I would
    /// do a thing and then did it" flow.
    fn handle(&self, incoming: &IncomingMessage) -> HandlerResult {
        let mut game = game!(self);
        let nick = incoming.user().unwrap().to_string();

        let captures = self.get_captures(incoming.get_contents()).unwrap();
        let reaction = captures.name("reaction");

        if !game.started {
            println!("Ignoring attempt to {:?} by {} - the game hasn't started", reaction, nick);
            return Ok(());
        }

        if game.state.action.is_none() {
            incoming.reply("No action to object to".to_string());
            return Ok(());
        }

        match reaction {
            Some("!bullshit") => {
                incoming.reply(format!("{} called bullshit", nick));
                game.state.bullshit = Some(nick);
            },
            _ => println!("Nfi what happened"),
        }

        Ok(())
    }
}
