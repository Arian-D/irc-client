use serde::de::SeqAccess;
use std::error::Error;
use std::fmt;
use std::io::{Read, Write};
use winnow::ascii::*;
use winnow::combinator::*;
use winnow::prelude::*;
use winnow::token::*;
use winnow::Result;

/// A low-level stateful struct representing the IRC client for a single network
#[derive(Debug)]
pub struct Client<'a, Socket>
where
    Socket: Read + Write,
{
    /// Server URI
    pub server: &'a str,
    /// Nickname
    pub nick: &'a str,
    /// Real name with spaces. Optional.
    pub real_name: Option<&'a str>,
    /// The IRC socket. It's most likely raw TCP or a TLS-wrapped one,
    /// but ¶8.1.1 from the RFC says that it could be a unix socket as
    /// well.
    pub socket: Socket,
    /// Auth
    pub auth: Auth<'a>,
}

impl<'a, Socket> Client<'a, Socket>
where Socket: Read + Write {
    pub fn send(&mut self, cmd: Command) {
        let _ = self.socket.write(format!("{cmd}").as_bytes());
    }

    pub fn read<'b>(&mut self, buf: &'b mut Vec<u8>) -> Vec<Message<'b>> {
        buf.clear();
        let mut staging = [0u8; 512];
        loop {
            match self.socket.read(&mut staging) {
                Ok(0) => break,
                Ok(n) => buf.extend_from_slice(&staging[..n]),
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        let input = std::str::from_utf8(buf).unwrap_or("");
        let mut result = vec![];
        let mut remaining = input;
        while let Ok(msg) = Message::parser(&mut remaining) {
            result.push(msg);
        }
        result
    }


}

/// A struct encapsulating IRC internal message information
#[derive(Debug, PartialEq)]
pub struct Message<'a> {
    pub tags: Option<Vec<&'a str>>,
    pub source: Option<&'a str>,
    pub command: &'a str,
    pub params: Option<Vec<&'a str>>,
}

/// An enum of all IRC commands
#[derive(Debug)]
pub enum Command {
    /// Nick message: Set nickname
    Nick { nickname: String },
    /// USER message: Set username and real name
    User {
        username: String,
        real_name: String,
    },
    /// QUIT the server with an optional message
    Quit { message: Option<String> },
    // TODO: Implement PASS
    /// JOIN 1 or more channels.
    Join { channels: Vec<String> },
    /// PART message: leave 1 or more channels
    Part { channels: Vec<String> },
    /// MODE message: Set the channel or user mode with args
    Mode { params: Vec<String> },
    /// TOPIC message: View or optionally set channel topic
    Topic {
        channel: String,
        topic: Option<String>,
    },
    /// NAMES: List NICKs, optionally providing channels
    Names { channels: Option<Vec<String>> },
    /// LISIT channel names
    List { channels: Option<Vec<String>> },
    /// INVITE user to channel
    Invite { user: String, channel: String },
    /// KICK: Kick user from channel with optional reason
    Kick {
        user: String,
        channel: String,
        reason: Option<String>,
    },
    /// PRVMSG: Send message to one or more receivers
    PrivMsg {
        message: String,
        receivers: Vec<String>,
    },
    // Commands for later
    // VERSION
    // STATS
    // LINKS
    // CONNECT
    // TRACE
    // ADMIN
    // INFO
    // WHO
    // WHOIS
    // WHOWAS
    // PONG
    // AWAY
}

impl Command {
    /// Convert command to Message struct
    fn command_to_message(&self) -> Message<'_> {
        match self {
            Command::Nick { nickname } => Message {
                tags: None,
                source: None,
                command: "NICK",
                params: Some(vec![nickname.as_str()]),
            },
            _ => todo!("😔"),
        }
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.command_to_message())
    }
}

impl<'a> fmt::Display for Message<'a> {
    /// https://modern.ircdocs.horse/#message-format
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Message {
            tags: _,
            source: prefix,
            command: command,
            params: params,
        } = self;
        write!(
            f,
            "{}{command}{}\r\n",
            if let Some(prefix) = prefix {
                format!(":{prefix} ")
            } else {
                "".to_string()
            },
            if let Some(args) = params {
                " ".to_string() + &args.join(" ")
            } else {
                "".to_string()
            },
        )
    }
}

impl<'a> Message<'a> {
    // TODO: Change this to read a stream directly instead of using str
    /// A parser for reading Messages.
    pub fn parser<'i>(i: &mut &'i str) -> ModalResult<Message<'i>> {
        seq! {
            Message {
                tags: opt(
                    preceded('@',
                        separated(
                            0..,
                            take_until(0.., ' '),
                            " "
                        )
                    )
                ),
                _: space0,
                source: opt(preceded(':', take_until(0.., ' '))),
                _: space0,
                // This might be incorrect
                command: alt((alpha1, digit1)),
                _: space0,
                params: opt(separated(0.., take_till(0.., |it| matches!(it, ' ' | '\r' | '\n')), " ")),
                _: line_ending,
            }
        }
        .parse_next(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parsing_source() {
        let mut input = "JOIN #foobar\r\n";
        assert_eq!(Message::parser(&mut input), Ok(Message {
            tags: None,
            source: None,
            command: "JOIN",
            params: Some(vec!["#foobar"])
        }))
    }
}

/// Authentication method
#[derive(Debug)]
pub enum Auth<'a> {
    /// NickServ with Nick and Pass (which may not exist)
    Plain(&'a str, Option<&'a str>),
    /// CertFP authentication. Unsure if this can be used in conjunction with the other, so it might need to be relocated.
    Cert(&'a str),
}
