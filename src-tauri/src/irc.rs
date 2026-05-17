use std::fmt;
use std::io::{Error, ErrorKind, Read, Write};
use std::time::{Duration, Instant};
use winnow::ascii::*;
use winnow::combinator::*;
use winnow::prelude::*;
use winnow::token::*;

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
    /// Bytes read from the socket that did not yet form a complete IRC line.
    pub(crate) read_buffer: Vec<u8>,
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
    use std::collections::VecDeque;
    use std::io;

    enum ReadStep {
        Data(&'static [u8]),
        WouldBlock,
    }

    struct FragmentedSocket {
        reads: VecDeque<ReadStep>,
        writes: Vec<u8>,
    }

    impl FragmentedSocket {
        fn new(reads: impl IntoIterator<Item = ReadStep>) -> Self {
            Self {
                reads: reads.into_iter().collect(),
                writes: Vec::new(),
            }
        }
    }

    impl Read for FragmentedSocket {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            match self.reads.pop_front().unwrap_or(ReadStep::WouldBlock) {
                ReadStep::Data(data) => {
                    let n = data.len().min(buf.len());
                    buf[..n].copy_from_slice(&data[..n]);
                    Ok(n)
                }
                ReadStep::WouldBlock => Err(io::Error::from(ErrorKind::WouldBlock)),
            }
        }
    }

    impl Write for FragmentedSocket {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.writes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn test_client(reads: impl IntoIterator<Item = ReadStep>) -> Client<'static, FragmentedSocket> {
        Client {
            server: "irc.example.test:6667",
            nick: "tester",
            real_name: None,
            socket: FragmentedSocket::new(reads),
            auth: Auth::None,
            read_buffer: Vec::new(),
        }
    }

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

    #[test]
    fn read_preserves_fragmented_irc_lines_between_polls() {
        let mut client = test_client([
            ReadStep::Data(b"PING :one\r\nPRIVMSG #room"),
            ReadStep::WouldBlock,
            ReadStep::Data(b" :hello\r\n"),
            ReadStep::WouldBlock,
        ]);
        let mut buf = Vec::new();

        let first_poll = client.read(&mut buf);
        assert_eq!(first_poll.len(), 1);
        assert_eq!(first_poll[0].command, "PING");
        assert_eq!(client.read_buffer, b"PRIVMSG #room".to_vec());

        let second_poll = client.read(&mut buf);
        assert_eq!(second_poll.len(), 1);
        assert_eq!(second_poll[0].command, "PRIVMSG");
        assert_eq!(second_poll[0].params, Some(vec!["#room", ":hello"]));
        assert!(client.read_buffer.is_empty());
    }

    #[test]
    fn read_waits_until_fragmented_line_is_complete() {
        let mut client = test_client([
            ReadStep::Data(b"PING :server"),
            ReadStep::WouldBlock,
            ReadStep::Data(b"\r\n"),
            ReadStep::WouldBlock,
        ]);
        let mut buf = Vec::new();

        assert!(client.read(&mut buf).is_empty());
        assert_eq!(client.read_buffer, b"PING :server".to_vec());

        let messages = client.read(&mut buf);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].command, "PING");
        assert!(client.read_buffer.is_empty());
    }
}

impl<'a, T: Read + Write> Client<'a, T> {
    pub fn read<'b>(&mut self, buf: &'b mut Vec<u8>) -> Vec<Message<'b>> {
        buf.clear();
        let mut staging = [0u8; 512];
        loop {
            match self.socket.read(&mut staging) {
                Ok(0) => break,
                Ok(n) => self.read_buffer.extend_from_slice(&staging[..n]),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(ref e) if read_would_block(e.kind()) => break,
                Err(_) => break,
            }
        }

        let complete_len = self
            .read_buffer
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        if complete_len == 0 {
            return Vec::new();
        }

        buf.extend(self.read_buffer.drain(..complete_len));

        let input = std::str::from_utf8(buf).unwrap_or("");
        let mut result = vec![];
        let mut remaining = input;
        while let Ok(msg) = Message::parser(&mut remaining) {
            result.push(msg);
        }
        result
    }

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

    pub fn service_connection(&mut self) -> std::io::Result<()> {
        let mut buf = Vec::new();
        let messages = self.read(&mut buf);
        for message in messages {
            if message.command == "PING" {
                if let Some(token) = message.params.and_then(|params| params.last().copied()) {
                    self.send_pong(token)?;
                }
                continue;
            }
            if message.command == "ERROR" {
                return Err(Error::new(
                    ErrorKind::ConnectionAborted,
                    "IRC server closed the connection",
                ));
            }
        }
        Ok(())
    }

    pub fn await_welcome(&mut self, timeout: Duration) -> std::io::Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            let mut buf = Vec::new();
            let messages = self.read(&mut buf);

            for message in messages {
                if message.command == "PING" {
                    if let Some(token) = message.params.and_then(|params| params.last().copied()) {
                        self.send_pong(token)?;
                    }
                    continue;
                }

                if message.command == "001" {
                    return Ok(());
                }

                if message.command == "433" {
                    return Err(Error::new(
                        ErrorKind::AddrInUse,
                        "Nickname already in use on this server",
                    ));
                }
            }

            std::thread::sleep(Duration::from_millis(50));
        }

        Err(Error::new(
            ErrorKind::TimedOut,
            "Timed out waiting for IRC registration confirmation",
        ))
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

    pub fn send_pong(&mut self, token: &str) -> std::io::Result<()> {
        self.write_raw_line(&format!("PONG {token}"))
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

fn read_would_block(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::WouldBlock | ErrorKind::TimedOut)
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
