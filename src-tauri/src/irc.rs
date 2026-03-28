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
            params: Some(vec![nick, "_", "_", name.unwrap_or(nick)])
        })
    }
}

/// Authentication method
enum Auth<'a> {
    /// NickServ with Nick and Pass (which may not exist)
    Plain(&'a str, Option<&'a str>),
    /// CertFP authentication. Unsure if this can be used in conjunction with the other, so it might need to be relocated.
    Cert(Option<&'a str>),
}
