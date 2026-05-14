use std::fmt;
use std::io::{Error, ErrorKind, Read, Write};
use winnow::ascii::*;
use winnow::combinator::*;
use winnow::prelude::*;
use winnow::token::*;
use winnow::Result;

/// A stateful struct representing the IRC client
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

/// A struct encapsulating IRC internal message information
#[derive(Debug, PartialEq)]
struct Message<'a> {
    tags: Option<Vec<&'a str>>,
    source: Option<&'a str>,
    command: &'a str,
    params: Option<Vec<&'a str>>,
}

/// An enum of all IRC commands
#[derive(Debug)]
enum Command<'a> {
    /// Nick message: Set nickname
    Nick { nickname: &'a str },
    /// USER message: Set username and real name
    User {
        username: &'a str,
        real_name: &'a str,
    },
    /// QUIT the server with an optional message
    Quit { message: Option<&'a str> },
    // TODO: Implement PASS
    /// JOIN 1 or more channels.
    Join { channels: Vec<&'a str> },
    /// PART message: leave 1 or more channels
    Part { channels: Vec<&'a str> },
    /// MODE message: Set the channel or user mode with args
    Mode { params: Vec<&'a str> },
    /// TOPIC message: View or optionally set channel topic
    Topic {
        channel: &'a str,
        topic: Option<&'a str>,
    },
    /// NAMES: List NICKs, optionally providing channels
    Names { channels: Option<Vec<&'a str>> },
    /// LISIT channel names
    List { channels: Option<Vec<&'a str>> },
    /// INVITE user to channel
    Invite { user: &'a str, channel: &'a str },
    /// KICK: Kick user from channel with optional reason
    Kick {
        user: &'a str,
        channel: &'a str,
        reason: Option<&'a str>,
    },
    /// PRVMSG: Send message to one or more receivers
    PrivMsg {
        message: &'a str,
        receivers: Vec<&'a str>,
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

impl<'a> Command<'a> {
    fn command_to_message(&self) -> Message<'a> {
        match self {
            Command::Nick { nickname: nickname } => Message {
                tags: None,
                source: None,
                command: "NICK",
                params: Some(vec![nickname]),
            },
            _ => todo!("😔"),
        }
    }
}

impl<'a> fmt::Display for Command<'a> {
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
    fn parser<'i>(i: &mut &'i str) -> ModalResult<Message<'i>> {
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
        assert_eq!(
            Message::parser(&mut input),
            Ok(Message {
                tags: None,
                source: None,
                command: "JOIN",
                params: Some(vec!["#foobar"])
            })
        )
    }
}

impl<'a, T: Read + Write> Client<'a, T> {
    fn write_raw_line(&mut self, line: &str) -> std::io::Result<()> {
        self.socket.write_all(line.as_bytes())?;
        self.socket.write_all(b"\r\n")?;
        self.socket.flush()
    }

    pub fn register_and_authenticate(&mut self) -> std::io::Result<()> {
        self.write_raw_line(&format!("NICK {}", self.nick))?;

        let username = self.nick;
        let real_name = self.real_name.unwrap_or(self.nick);
        self.write_raw_line(&format!("USER {username} 0 * :{real_name}"))?;

        if let Auth::NickServ { account, password } = &self.auth {
            let identify_command = if let Some(account) = account {
                format!("PRIVMSG NickServ :IDENTIFY {account} {password}")
            } else {
                format!("PRIVMSG NickServ :IDENTIFY {password}")
            };
            self.write_raw_line(&identify_command)?;
        }

        Ok(())
    }

    pub fn join_channel(&mut self, channel: &str) -> std::io::Result<()> {
        let channel = sanitize_irc_line_value(channel);
        if channel.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Channel cannot be empty",
            ));
        }
        self.write_raw_line(&format!("JOIN {channel}"))
    }

    pub fn leave_channel(&mut self, channel: &str) -> std::io::Result<()> {
        let channel = sanitize_irc_line_value(channel);
        if channel.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Channel cannot be empty",
            ));
        }
        self.write_raw_line(&format!("PART {channel}"))
    }

    pub fn send_privmsg(&mut self, receiver: &str, message: &str) -> std::io::Result<()> {
        let receiver = sanitize_irc_line_value(receiver);
        let message = sanitize_irc_line_value(message);
        if receiver.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Receiver cannot be empty",
            ));
        }
        if message.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Message cannot be empty",
            ));
        }
        self.write_raw_line(&format!("PRIVMSG {receiver} :{message}"))
    }

    fn read_from_socket(&mut self) -> String {
        let mut result: String = String::new();
        // Based on the ¶8.2
        let mut buffer = vec![0; 512];
        while let Ok(_) = self.socket.read_exact(&mut buffer) {
            result += str::from_utf8(&buffer).unwrap();
        }
        result
    }
}

fn sanitize_irc_line_value(value: &str) -> String {
    value
        .chars()
        .filter(|ch| *ch != '\r' && *ch != '\n')
        .collect::<String>()
        .trim()
        .to_string()
}

/// Authentication method
#[derive(Debug)]
pub enum Auth<'a> {
    /// No explicit authentication.
    None,
    /// Authenticate to NickServ after registration.
    NickServ {
        account: Option<&'a str>,
        password: &'a str,
    },
    /// CertFP authentication. Unsure if this can be used in conjunction with the other, so it might need to be relocated.
    Cert(&'a str),
}
