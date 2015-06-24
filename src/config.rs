use std::io;
use std::io::{Read,Write};
use std::convert;
use std::fs::File;

use rustc_serialize::json;

use chatbot::adapter::{IrcAdapter,IrcConfig};


// Named kinda weirdly to avoid colliding with the chatbots notion of config
#[derive(RustcDecodable)]
pub struct CoupIrcConfig {
    nick: String,
    server: String,
    channels: Vec<String>,
    tls: bool,
}

impl CoupIrcConfig {
    pub fn from_file(filename: &str) -> io::Result<CoupIrcConfig> {
        let mut fh = try!(File::open(filename));
        let mut body = String::new();
        let _ = fh.read_to_string(&mut body);
        // TODO(richo) Deal more gracefully with this condition
        Ok(json::decode(&body).unwrap())
    }
}

impl convert::Into<IrcConfig> for CoupIrcConfig {
    fn into(self) -> IrcConfig {
        IrcConfig {
            nickname: Some(self.nick),
            server: Some(self.server),
            channels: Some(self.channels),
            use_ssl: Some(self.tls),
            .. Default::default()
        }
    }
}
