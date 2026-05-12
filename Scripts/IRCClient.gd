extends Node

signal connected_to_server(server : String)
signal disconnected_from_server()
signal message_received(parsed : Dictionary)
signal channel_joined(channel : String)
signal channel_parted(channel : String)
signal nick_changed(old_nick : String, new_nick : String)
signal capability_acknowledged(caps : Array)
signal error_received(code : int, message : String)
signal server_motd(text : String)
signal userlist_received(channel : String, nicks : Array)
signal user_joined(channel : String, nick : String, hostmask : String)
signal user_parted(channel : String, nick : String, reason : String)
signal user_quit(nick : String, reason : String)
signal topic_received(channel : String, topic : String)
signal privmsg_received(source : String, target : String, text : String)
signal notice_received(source : String, target : String, text : String)
signal ctcp_received(source : String, command : String, params : String)
signal ping_received(token : String)

const DESIRED_CAPS : Array = [
	"multi-prefix", "away-notify", "account-notify", "extended-join",
	"server-time", "message-tags", "batch", "labeled-response",
	"echo-message", "userhost-in-names", "cap-notify", "sasl", ]

var socket : StreamPeerTCP = StreamPeerTCP.new()
var buffer : String = ""
var connected : bool = false
var registered : bool = false

var server : String = ""
var port : int = 6667
var nickname : String = ""
var username : String = ""
var realname : String = ""
var password : String = ""

var channels : Dictionary = {}  # channel -> {topic, nicks, modes}
var acknowledged_caps : Array = []
var server_info : Dictionary = {}
var pending_caps : bool = false
var join_queue: Array = []  # channels to join once registered
var cap_ls_buffer: Array = []  # accumulates multi-line CAP LS 302 responses

func _ready() -> void:
	set_process(true)

func _physics_process(_delta : float) -> void:
	if socket == null:
		return

	socket.poll()

	match socket.get_status():
		StreamPeerTCP.STATUS_CONNECTED:
			if connected == false:
				print("Finally Connected")
				connected = true
				print("begining registration")
				begin_registration()
			receive_data()
		StreamPeerTCP.STATUS_CONNECTING:
			print("trying to connect")
			pass
		StreamPeerTCP.STATUS_NONE, StreamPeerTCP.STATUS_ERROR:
			if connected == true:
				connected = false
				push_warning("Lost or Failed Connection")

func receive_data() -> void:
	while socket.get_available_bytes() > 0:
		var result = socket.get_utf8_string(socket.get_available_bytes())
		if result == "":
			break

		for line in result.split("\r\n", false):
			if line != "":
				handle_line(line)

# --- Connection ---------------------------------------------------------------
func connect_to_server(p_server : String, p_port : int, p_nick : String, p_user : String, p_real : String, p_pass : String) -> void:
	print("Called Connecting to server")
	
	server   = p_server
	port     = p_port
	nickname = p_nick
	username = p_user
	realname = p_real
	password = p_pass

	buffer  = ""
	registered = false
	acknowledged_caps.clear()
	channels.clear()
	cap_ls_buffer.clear()
	join_queue.clear()

	var err : int = socket.connect_to_host(server, port)

	print("SOCKET STATUS: " + str(socket.get_status()))
	print("Socket host: " + socket.get_connected_host())
	print("Socket port: " + str(socket.get_connected_port()))
	print("Local socket port: " + str(socket.get_local_port()))
	
	if err != OK:
		push_error("IRCClient: TCP connect failed: %s" % error_string(err))
		return

	emit_signal("connected_to_server", server)

func disconnect_from_server(quit_message: String = "Goodbye") -> void:
	if connected:
		send_raw("QUIT :%s" % quit_message)
	socket.disconnect_from_host()
	connected = false
	registered = false
	emit_signal("disconnected_from_server")

func is_connected_to_server() -> bool:
	return connected

# --- Registration -------------------------------------------------------------
func begin_registration() -> void:
	send_raw("CAP LS 302") # IRCv3 capability negotiation with this special phrase 

	if password != "":
		send_raw("PASS :%s" % password)

	send_raw("NICK %s" % nickname)
	send_raw("USER %s 0 * :%s" % [username, realname])

# --- Sending ------------------------------------------------------------------
func send_raw(line : String) -> void:
	if socket == null or socket.get_status() != StreamPeerTCP.STATUS_CONNECTED:
		push_warning("IRC Client Not Connected, cant send: " + line)

	var data : PackedByteArray = (line + "\r\n").to_utf8_buffer()
	var error : int = socket.put_data(data)
	
	if error != OK:
		push_warning("IRC Client: Error sending data. Code: " + str(error) + " Line: " + line)

func send_privmsg(target: String, text: String) -> void:
	send_raw("PRIVMSG %s :%s" % [target, text])

func send_notice(target: String, text: String) -> void:
	send_raw("NOTICE %s :%s" % [target, text])

func send_ctcp(target: String, command: String, params: String = "") -> void: # Client to client protocol
	var msg := "%s%s%s%s" % [char(1), command, (" " + params) if params else "", char(1)]
	send_raw("PRIVMSG %s :%s" % [target, msg])

func join_channel(channel: String, key: String = "") -> void:
	if not registered:
		join_queue.append({"channel": channel, "key": key})
		print("registering if not registered")
		return
	if key:
		send_raw("JOIN %s %s" % [channel, key])
		print("join key")
	else:
		send_raw("JOIN %s" % channel)
		print("join without key")

func part_channel(channel: String, reason: String = "") -> void:
	if reason:
		send_raw("PART %s :%s" % [channel, reason])
	else:
		send_raw("PART %s" % channel)

func set_nick(new_nick: String) -> void:
	send_raw("NICK %s" % new_nick)

func set_topic(channel: String, topic: String) -> void:
	send_raw("TOPIC %s :%s" % [channel, topic])

func request_topic(channel: String) -> void:
	send_raw("TOPIC %s" % channel)

func kick(channel: String, nick: String, reason: String = "") -> void:
	if reason:
		send_raw("KICK %s %s :%s" % [channel, nick, reason])
	else:
		send_raw("KICK %s %s" % [channel, nick])

func send_ping(token: String = "") -> void:
	if token == "":
		token = str(Time.get_ticks_msec())
	send_raw("PING :%s" % token)

# --- IRCv3 Parsing ------------------------------------------------------------
## Returns a Dictionary:
##   tags   : Dictionary  (IRCv3 message tags)
##   prefix : String      (nick!user@host or server)
##   nick   : String
##   command: String
##   params : Array[String]
##   trailing: String
static func parse_message(raw : String) -> Dictionary:
	var result : Dictionary = { "tags": {}, "prefix": "", "nick": "", "user": "", 
	"host": "", "command": "", "params": [],"trailing": "", }

	var pos := 0
	if raw.begins_with("@"): # Tags (@key=value;key2=value2)
		var space := raw.find(" ", 1)
		if space == -1:
			return result
		var tag_str := raw.substr(1, space - 1)
		for tag in tag_str.split(";"):
			var kv := tag.split("=", true, 1)
			result["tags"][kv[0]] = kv[1] if kv.size() > 1 else ""
		pos = space + 1

	# Skip whitespace
	while pos < raw.length() and raw[pos] == " ":
		pos += 1

	# Prefix (:nick!user@host or :server)
	if pos < raw.length() and raw[pos] == ":":
		pos += 1
		var space := raw.find(" ", pos)
		if space == -1:
			return result
		var prefix := raw.substr(pos, space - pos)
		result["prefix"] = prefix
		if "!" in prefix:
			var parts := prefix.split("!", true, 1)
			result["nick"] = parts[0]
			if "@" in parts[1]:
				var uh := parts[1].split("@", true, 1)
				result["user"] = uh[0]
				result["host"] = uh[1]
		else:
			result["nick"] = prefix
		pos = space + 1

	# Skip whitespace
	while pos < raw.length() and raw[pos] == " ":
		pos += 1

	# Command
	var space2 := raw.find(" ", pos)
	if space2 == -1:
		result["command"] = raw.substr(pos).to_upper()
		return result
	result["command"] = raw.substr(pos, space2 - pos).to_upper()
	pos = space2 + 1

	# Params
	while pos < raw.length():
		while pos < raw.length() and raw[pos] == " ":
			pos += 1
		if pos >= raw.length():
			break
		if raw[pos] == ":":
			result["trailing"] = raw.substr(pos + 1)
			result["params"].append(result["trailing"])
			break
		else:
			var sp := raw.find(" ", pos)
			if sp == -1:
				result["params"].append(raw.substr(pos))
				break
			else:
				result["params"].append(raw.substr(pos, sp - pos))
				pos = sp + 1

	return result

# --- Dispatch -----------------------------------------------------------------
func handle_line(line: String) -> void:
	var msg := parse_message(line)
	emit_signal("message_received", msg)
	
	print("[IRC RAW] ", line) # Debugging of registration

	var cmd : String = msg["command"]

	match cmd:
		"PING":         on_ping(msg)
		"CAP":          on_cap(msg)
		"001":          on_001(msg)
		"002":          pass
		"003":          pass
		"004":          on_004(msg)
		"005":          on_005(msg)
		"375":          emit_signal("server_motd", msg["trailing"])
		"372":          emit_signal("server_motd", msg["trailing"])
		"376":          on_376(msg)
		"353":          on_353(msg)
		"366":          on_366(msg)
		"332":          on_332(msg)
		"333":          pass
		"433":          on_433(msg)
		"JOIN":         on_join(msg)
		"PART":         on_part(msg)
		"QUIT":         on_quit(msg)
		"NICK":         on_nick(msg)
		"PRIVMSG":      on_privmsg(msg)
		"NOTICE":       on_notice(msg)
		"TOPIC":        on_topic(msg)
		"KICK":         on_kick(msg)
		"ERROR":        on_error(msg)
		_:
			if cmd.is_valid_int():
				var code := int(cmd)
				if code >= 400:
					emit_signal("error_received", code, msg["trailing"])

# --- Handlers -----------------------------------------------------------------
func on_ping(msg : Dictionary) -> void:
	var token: String = msg["trailing"] if msg["trailing"] != "" else msg["params"][0] if msg["params"].size() > 0 else ""
	send_raw("PONG :%s" % token)
	emit_signal("ping_received", token)

func on_cap(msg : Dictionary) -> void:
	var subcmd: String = msg["params"][1].to_upper() if msg["params"].size() > 1 else ""
	match subcmd:
		"LS":
			# CAP LS 302 can be multi-line; a * in params[2] means more lines coming
			print("[CAP LS] params=", msg["params"], " trailing=", msg["trailing"])
			var is_multiline: bool = msg["params"].size() > 2 and msg["params"][2] == "*"
			print("[CAP LS] is_multiline=", is_multiline)
			# Accumulate cap tokens, stripping any =value suffixes
			for token in msg["trailing"].split(" "):
				var cap_name: String = token.split("=")[0].strip_edges()
				if cap_name != "":
					cap_ls_buffer.append(cap_name)
			if is_multiline:
				return  # wait for the final LS line
			# Final LS line received — now request what we want
			var req := PackedStringArray()
			for c in DESIRED_CAPS:
				if c in cap_ls_buffer:
					req.append(c)
			cap_ls_buffer.clear()
			if req.size() > 0:
				pending_caps = true
				send_raw("CAP REQ :%s" % " ".join(req))
			else:
				send_raw("CAP END")
		"ACK":
			for c in msg["trailing"].split(" "):
				var cap: String = c.strip_edges()
				if cap and cap not in acknowledged_caps:
					acknowledged_caps.append(cap)
			emit_signal("capability_acknowledged", acknowledged_caps)
			pending_caps = false
			send_raw("CAP END")
		"NAK":
			pending_caps = false
			send_raw("CAP END")

func on_001(msg : Dictionary) -> void: # on welcome this runs
	registered = true
	nickname = msg["params"][0]
	# Flush any channels queued before registration completed
	for entry in join_queue:
		join_channel(entry["channel"], entry["key"])
	join_queue.clear()

func on_004(msg : Dictionary) -> void: # server version usermode channelmode
	server_info["server_name"] = msg["params"][1] if msg["params"].size() > 1 else ""

func on_005(msg : Dictionary) -> void: # server map
	for param in msg["params"]:
		if "=" in param:
			var kv: PackedStringArray = param.split("=", true, 1)
			server_info[kv[0]] = kv[1]
		elif param != nickname:
			server_info[param] = true

func on_376(_msg : Dictionary) -> void: # Channel ban nick time # MOTD End — fully registered
	pass

func on_353(msg : Dictionary) -> void: # Channel names
	# RPL_NAMREPLY  :server 353 me = #channel :nick1 nick2 ...
	var channel: String = msg["params"][2] if msg["params"].size() > 2 else ""
	if channel == "":
		return
	if channel not in channels:
		channels[channel] = {"topic": "", "nicks": [], "modes": ""}
	var nicks_raw: PackedStringArray = msg["trailing"].split(" ")
	for n in nicks_raw:
		var nick: String = n.strip_edges()
		if nick and nick not in channels[channel]["nicks"]:
			channels[channel]["nicks"].append(nick)

func on_366(msg : Dictionary) -> void: # channel end of /NAMES list
	var channel : String = msg["params"][1] if msg["params"].size() > 1 else ""
	if channel in channels:
		emit_signal("userlist_received", channel, channels[channel]["nicks"].duplicate())

func on_332(msg : Dictionary) -> void: # channel users topic
	var channel : String = msg["params"][1] if msg["params"].size() > 1 else ""
	var topic : String  = msg["trailing"]

	if channel in channels:
		channels[channel]["topic"] = topic

	emit_signal("topic_received", channel, topic)

func on_433(_msg : Dictionary) -> void: # Nickname already in use
	nickname = nickname + "_"
	send_raw("NICK %s" % nickname)

func on_join(msg: Dictionary) -> void:
	var channel : String = msg["params"][0] if msg["params"].size() > 0 else msg["trailing"]
	var nick : String = msg["nick"]
	var hostmask : String = msg["prefix"]

	if nick == nickname:
		if channel not in channels:
			channels[channel] = {"topic": "", "nicks": [], "modes": ""}
		emit_signal("channel_joined", channel)
	else:
		if channel in channels and nick not in channels[channel]["nicks"]:
			channels[channel]["nicks"].append(nick)
		emit_signal("user_joined", channel, nick, hostmask)

func on_part(msg : Dictionary) -> void:
	var channel : String = msg["params"][0] if msg["params"].size() > 0 else ""
	var nick : String    = msg["nick"]
	var reason : String  = msg["trailing"]

	if nick == nickname:
		channels.erase(channel)
		emit_signal("channel_parted", channel)
	else:
		if channel in channels:
			channels[channel]["nicks"].erase(nick)
		emit_signal("user_parted", channel, nick, reason)

func on_quit(msg : Dictionary) -> void:
	var nick : String   = msg["nick"]
	var reason : String = msg["trailing"]
	for ch in channels:
		channels[ch]["nicks"].erase(nick)
	emit_signal("user_quit", nick, reason)

func on_nick(msg : Dictionary) -> void:
	var old_nick : String = msg["nick"]
	var new_nick : String = msg["trailing"] if msg["trailing"] != "" else msg["params"][0]
	for ch in channels:
		var idx: int = channels[ch]["nicks"].find(old_nick)
		if idx != -1:
			channels[ch]["nicks"][idx] = new_nick
	if old_nick == nickname:
		nickname = new_nick
	emit_signal("nick_changed", old_nick, new_nick)

func on_privmsg(msg: Dictionary) -> void:
	var source : String = msg["nick"]
	var target : String = msg["params"][0] if msg["params"].size() > 0 else ""
	var text : String   = msg["trailing"]
	# CTCP detection (char(1) = 0x01, GDScript does not support \x01 escapes)
	var ctcp_delim := char(1)
	if text.begins_with(ctcp_delim) and text.ends_with(ctcp_delim):
		var inner := text.substr(1, text.length() - 2)
		var sp := inner.find(" ")
		var ctcp_cmd    := inner.substr(0, sp if sp != -1 else inner.length())
		var ctcp_params := inner.substr(sp + 1) if sp != -1 else ""
		if ctcp_cmd == "ACTION":
			emit_signal("privmsg_received", source, target, "* %s %s" % [source, ctcp_params])
		else:
			emit_signal("ctcp_received", source, ctcp_cmd, ctcp_params)
			if ctcp_cmd == "VERSION":
				send_raw("NOTICE %s :%sVERSION Godot IRCv3 Client 1.0%s" % [source, ctcp_delim, ctcp_delim])
			elif ctcp_cmd == "PING":
				send_raw("NOTICE %s :%sPING %s%s" % [source, ctcp_delim, ctcp_params, ctcp_delim])
		return
	emit_signal("privmsg_received", source, target, text)

func on_notice(msg : Dictionary) -> void:
	var source : String = msg["nick"]
	var target : String = msg["params"][0] if msg["params"].size() > 0 else ""
	var text : String   = msg["trailing"]
	emit_signal("notice_received", source, target, text)

func on_topic(msg : Dictionary) -> void:
	var channel: String = msg["params"][0] if msg["params"].size() > 0 else ""
	var topic: String   = msg["trailing"]
	if channel in channels:
		channels[channel]["topic"] = topic
	emit_signal("topic_received", channel, topic)

func on_kick(msg : Dictionary) -> void:
	var channel: String  = msg["params"][0] if msg["params"].size() > 0 else ""
	var kicked: String   = msg["params"][1] if msg["params"].size() > 1 else ""
	var reason: String   = msg["trailing"]
	if channel in channels:
		channels[channel]["nicks"].erase(kicked)
	if kicked == nickname:
		channels.erase(channel)
		emit_signal("channel_parted", channel)
	else:
		emit_signal("user_parted", channel, kicked, "Kicked: " + reason)

func on_error(msg: Dictionary) -> void:
	emit_signal("error_received", 0, msg["trailing"])
	connected = false
	emit_signal("disconnected_from_server")
