use std::error::Error;
use std::io::{Read, Write};

/// A stateful struct representing the IRC client
pub struct Client<'a, T: Read + Write> {
    /// Server URI
    server: &'a str,
    /// Nickname
    nick: &'a str,
    /// Real name with spaces. Optional.
    real_name: Option<&'a str>,
    /// The IRC socket. It's most likely raw TCP or a TLS-wrapped one,
    /// but ¶8.1.1 from the RFC says that it could be a unix socket as
    /// well.
    socket: T,
}

#[derive(Debug)]
struct Message<'a> {
    tags: Option<Vec<&'a str>>,
    prefix: Option<&'a str>,
    command: &'a str,
    params: Option<Vec<&'a str>>,
}

enum Command<'a> {
    /// Nick message: Set nickname
    Nick(&'a str),
    /// USER message: Set username and real name
    User(&'a str, &'a str),
    /// QUIT the server with an optional message
    Quit(Option<&'a str>),
    // TODO: Implement passwords
    /// JOIN 1 or more channels.
    Join(Vec<&'a str>),
    /// PART message: leave 1 or more channels
    Part(Vec<&'a str>),
    /// MODE message: Set the channel or user mode with args
    Mode(Vec<&'a str>),
    /// TOPIC message: View or optionally set channel topic
    Topic(&'a str, Option<&'a str>),
    /// NAMES: List NICKs, optionally providing channels
    Names(Vec<&'a str>),
    /// LISIT channel names
    List(Vec<&'a str>),
    /// INVITE user to channel
    Invite(&'a str, &'a str),
    /// KICK: Kick user (2) from channel (1) with optional reason (3)
    Kick(&'a str, &'a str, Option<&'a str>),
    /// PRVMSG: Send message (2) to one or more receivers (1)
    PrivMsg(Vec<&'a str>, String)
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




impl<'a, T: Read + Write> Client<'a, T> {
    fn send_raw_command(mut self, message: Message<'_>) {
        if let Message {
            tags: _,
            prefix: prefix,
            command: command,
            params: params,
        } = message
        {
            self.socket.write(
                format!(
                    "{}{}{}\r\n",
                    if let Some(prefix) = prefix {
                        format!(":{prefix} ")
                    } else {
                        "".to_string()
                    },
                    command,
                    if let Some(args) = params {
                        " ".to_string() + &args.join(" ")
                    } else {
                        "".to_string()
                    },
                )
                .as_bytes(),
            );
        }
    }

    fn user_command(mut self) {
        let nick = self.nick;
        let name = self.real_name;
        self.send_raw_command(Message {
            tags: None,
            prefix: None,
            command: "USER",
            params: Some(vec![nick, "_", "_", name.unwrap_or(nick)]),
        })
    }

    fn nick_command(mut self) {
        let nick = self.nick;
        self.send_raw_command(Message {
            tags: None,
            prefix: None,
            command: "NICK",
            params: Some(vec![nick]),
        })
    }
}

/// Authentication method
enum Auth<'a> {
    /// NickServ with Nick and Pass (which may not exist)
    Plain(&'a str, Option<&'a str>),
    /// CertFP authentication. Unsure if this can be used in conjunction with the other, so it might need to be relocated.
    Cert(&'a str),
}
