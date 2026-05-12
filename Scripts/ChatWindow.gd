extends Control

@onready var tab_container : TabContainer = $VBox/HBox/TabContainer
@onready var user_list : ItemList = $VBox/HBox/UserListPanel/UserList
@onready var input_field : LineEdit = $VBox/InputRow/InputField
@onready var send_btn : Button = $VBox/InputRow/SendBtn
@onready var status_bar : Label = $VBox/StatusBar
@onready var connect_dialog : Window = $ConnectDialog
@onready var topic_bar : Label = $VBox/TopicBar

var chat_logs : Dictionary = {} ## Map channel/server -> RichTextLabel (the chat log)
var active_target : String = "Status"

const COLOR_SERVER  : Color = Color(0.0, 0.537, 0.895, 1.0)
const COLOR_SELF    : Color = Color(0.0, 0.59, 0.197, 1.0)
const COLOR_MENTION : Color = Color(0.902, 0.809, 0.247, 1.0)
const COLOR_NOTICE  : Color = Color(0.895, 0.521, 0.147, 1.0)
const COLOR_ERROR   : Color = Color(0.926, 0.242, 0.242, 1.0)
const COLOR_JOIN    : Color = Color(0.255, 0.816, 0.0, 1.0)
const COLOR_PART    : Color = Color(0.898, 0.44, 0.137, 1.0)
const COLOR_DEFAULT : Color = Color(0.531, 0.531, 0.531, 1.0)
const COLOR_NICK    : Color = Color(0.277, 0.711, 1.0, 1.0)

func _ready() -> void:
	create_tab("Status")
	connect_irc_signals()

	input_field.grab_focus()
	send_btn.pressed.connect(on_send_pressed)
	input_field.text_submitted.connect(on_send_submitted)
	tab_container.tab_changed.connect(on_tab_changed)
	connect_dialog.popup_centered()
	set_status("Not connected")

# --- Tab Management -----------------------------------------------------------
func create_tab(name : String) -> RichTextLabel:
	if name in chat_logs:
		return chat_logs[name]
		
	var rtl : RichTextLabel = RichTextLabel.new()
	rtl.bbcode_enabled  = true
	rtl.scroll_following = true
	rtl.size_flags_vertical = Control.SIZE_EXPAND_FILL
	rtl.selection_enabled = true
	tab_container.add_child(rtl)
	tab_container.set_tab_title(tab_container.get_tab_count() - 1, name)
	chat_logs[name] = rtl

	return rtl

func remove_tab(name : String) -> void:
	if name not in chat_logs:
		return

	var rtl : RichTextLabel = chat_logs[name]
	tab_container.remove_child(rtl)
	rtl.queue_free()
	chat_logs.erase(name)

func focus_tab(name: String) -> void:
	if name not in chat_logs:
		create_tab(name)

	for i in tab_container.get_tab_count():
		if tab_container.get_tab_title(i) == name:
			tab_container.current_tab = i
			active_target = name
			refresh_userlist()
			refresh_topic()
			break

func on_tab_changed(idx: int) -> void:
	active_target = tab_container.get_tab_title(idx)
	refresh_userlist()
	refresh_topic()

# --- Printing -----------------------------------------------------------------
func timestamp() -> String:
	return "[%s]" % Time.get_time_string_from_system().substr(0, 8)

func append(target: String, text: String, color: Color = COLOR_DEFAULT) -> void:
	if target not in chat_logs:
		create_tab(target)

	var rtl : RichTextLabel = chat_logs[target]
	var hex := "#%02x%02x%02x" % [int(color.r * 255), int(color.g * 255), int(color.b * 255)]
	rtl.append_text("[color=black]%s[/color] [color=%s]%s[/color]\n" % [timestamp(), hex, text.xml_escape()])

func append_chat(target: String, nick: String, text: String) -> void:
	var is_me : bool = nick == IRCClient.nickname
	var mention : bool = IRCClient.nickname.to_lower() in text.to_lower()

	var nick_color : Color = nick_color(nick)
	var msg_color  : Color = COLOR_MENTION if mention else COLOR_DEFAULT

	if is_me:
		msg_color = COLOR_SELF

	var hex_nick := "#%02x%02x%02x" % [int(nick_color.r * 255), int(nick_color.g * 255), int(nick_color.b * 255)]
	var hex_msg  := "#%02x%02x%02x" % [int(msg_color.r * 255), int(msg_color.g * 255), int(msg_color.b * 255)]

	var rtl: RichTextLabel = chat_logs.get(target, chat_logs.get("Status"))

	rtl.append_text(
		"[color=black]%s[/color] [color=%s]<%s>[/color] [color=%s]%s[/color]\n"
		% [timestamp(), hex_nick, nick.xml_escape(), hex_msg, text.xml_escape()]
	)

func nick_color(nick: String) -> Color:
	var h : float = 0.0
	for ch in nick:
		h = fmod(h + ch.unicode_at(0) * 0.0613, 1.0)
	return Color.from_hsv(h, 0.65, 0.9)

# --- IRC Signal Handlers ------------------------------------------------------
func connect_irc_signals() -> void:
	IRCClient.connected_to_server.connect(on_irc_connected)
	IRCClient.disconnected_from_server.connect(on_irc_disconnected)
	IRCClient.server_motd.connect(on_motd)
	IRCClient.channel_joined.connect(on_channel_joined)
	IRCClient.channel_parted.connect(on_channel_parted)
	IRCClient.user_joined.connect(on_user_joined)
	IRCClient.user_parted.connect(on_user_parted)
	IRCClient.user_quit.connect(on_user_quit)
	IRCClient.nick_changed.connect(on_nick_changed)
	IRCClient.privmsg_received.connect(on_privmsg)
	IRCClient.notice_received.connect(on_notice)
	IRCClient.topic_received.connect(on_topic)
	IRCClient.capability_acknowledged.connect(on_caps_acked)
	IRCClient.error_received.connect(on_irc_error)
	IRCClient.userlist_received.connect(on_userlist)
	IRCClient.ctcp_received.connect(on_ctcp)

func on_irc_connected(server: String) -> void:
	append("Status", "Connected to %s" % server, COLOR_SERVER)
	set_status("Connected — %s" % server)

func on_irc_disconnected() -> void:
	append("Status", "Disconnected.", COLOR_ERROR)
	set_status("Disconnected")

func on_motd(text : String) -> void:
	append("Status", text, COLOR_SERVER)

func on_channel_joined(channel : String) -> void:
	create_tab(channel)
	focus_tab(channel)
	append(channel, "Joined %s" % channel, COLOR_JOIN)

func on_channel_parted(channel: String) -> void:
	append(channel, "You left %s" % channel, COLOR_PART)
	await get_tree().create_timer(1.5).timeout
	remove_tab(channel)
	focus_tab("Status")

func on_user_joined(channel: String, nick: String, _hostmask: String) -> void:
	append(channel, "→ %s joined" % nick, COLOR_JOIN)
	refresh_userlist()

func on_user_parted(channel: String, nick: String, reason: String) -> void:
	var msg := "← %s left" % nick
	if reason != "":
		msg += " (%s)" % reason
	append(channel, msg, COLOR_PART)
	refresh_userlist()

func on_user_quit(nick: String, reason: String) -> void:
	for ch in IRCClient.channels:
		if nick in IRCClient.channels[ch]["nicks"]:
			var msg := "← %s quit" % nick
			if reason != "":
				msg += " (%s)" % reason
			append(ch, msg, COLOR_PART)
	refresh_userlist()

func on_nick_changed(old_nick: String, new_nick: String) -> void:
	for ch in IRCClient.channels:
		append(ch, "• %s is now known as %s" % [old_nick, new_nick], COLOR_NOTICE)
	if old_nick == IRCClient.nickname:
		set_status("Nick: %s" % new_nick)
	refresh_userlist()

func on_privmsg(source: String, target: String, text: String) -> void:
	var dest := target if target.begins_with("#") or target.begins_with("&") else source
	if dest not in chat_logs:
		create_tab(dest)
	append_chat(dest, source, text)

func on_notice(source: String, _target: String, text: String) -> void:
	append("Status", "[NOTICE %s] %s" % [source, text], COLOR_NOTICE)

func on_topic(channel: String, topic: String) -> void:
	append(channel, "Topic: %s" % topic, COLOR_SERVER)
	if active_target == channel:
		topic_bar.text = "Topic: " + topic

func on_caps_acked(caps: Array) -> void:
	append("Status", "IRCv3 caps: %s" % ", ".join(caps), COLOR_SERVER)

func on_irc_error(code: int, message: String) -> void:
	var label := "ERROR" if code == 0 else "ERROR %d" % code
	append("Status", "[%s] %s" % [label, message], COLOR_ERROR)

func on_userlist(channel: String, nicks: Array) -> void:
	if active_target == channel:
		populate_userlist(nicks)

func on_ctcp(source: String, command: String, params: String) -> void:
	append("Status", "[CTCP] %s → %s %s" % [source, command, params], COLOR_NOTICE)

# --- Userlist -----------------------------------------------------------------
func refresh_userlist() -> void:
	if active_target in IRCClient.channels:
		populate_userlist(IRCClient.channels[active_target]["nicks"])
	else:
		user_list.clear()

func populate_userlist(nicks : Array) -> void:
	user_list.clear()
	for nick in nicks:
		user_list.add_item(nick)

func refresh_topic() -> void:
	if active_target in IRCClient.channels:
		topic_bar.text = "Topic: " + IRCClient.channels[active_target].get("topic", "")
	else:
		topic_bar.text = ""

# --- Input --------------------------------------------------------------------
func on_send_pressed() -> void:
	_process_input(input_field.text)
	input_field.clear()

func on_send_submitted(text: String) -> void:
	_process_input(text)
	input_field.clear()

func _process_input(raw: String) -> void:
	var text : String = raw.strip_edges()
	if text == "":
		return

	if text.begins_with("/"):
		handle_command(text)
	else:
		if not IRCClient.is_connected_to_server():
			append("Status", "Not connected.", COLOR_ERROR)
			return
		if active_target == "Status":
			append("Status", "Choose a channel or open a query first.", COLOR_ERROR)
			return

		IRCClient.send_privmsg(active_target, text)
		append_chat(active_target, IRCClient.nickname, text)

func handle_command(raw: String) -> void:
	var parts : PackedStringArray = raw.substr(1).split(" ", false)

	if parts.is_empty():
		return

	var cmd : String = parts[0].to_upper()
	var args : PackedStringArray = parts.slice(1)
	var rest : String = " ".join(args)

	match cmd:
		"CONNECT":
			connect_dialog.popup_centered()
		"JOIN":
			if args.size() > 0:
				if not IRCClient.is_connected_to_server():
					append("Status", "Not connected to a server.", COLOR_ERROR)
				else:
					IRCClient.join_channel(args[0]) #IRCClient.join_channel(args[0], args[1] if args.size() > 1 else "")
					print(args[0])
					
					if IRCClient.registered == false:
						append("Status", "Queued join for %s (waiting for server registration)..." % args[0], COLOR_SERVER)
			else:
				append("Status", "Usage: /join #channel [key]", COLOR_ERROR)
		"PART":
			var target := args[0] if args.size() > 0 else active_target
			IRCClient.part_channel(target, rest.substr(args[0].length()).strip_edges() if args.size() > 0 else "")
		"QUIT":
			IRCClient.disconnect_from_server(rest if rest != "" else "Goodbye")
		"MSG", "QUERY":
			if args.size() > 1:
				var target : String = args[0]
				var msg : String = " ".join(args.slice(1))
				IRCClient.send_privmsg(target, msg)
				if target not in chat_logs:
					create_tab(target)
				append_chat(target, IRCClient.nickname, msg)
				focus_tab(target)
			else:
				append("Status", "Usage: /msg <nick> <message>", COLOR_ERROR)
		"NICK":
			if args.size() > 0:
				IRCClient.set_nick(args[0])
			else:
				append("Status", "Usage: /nick <newnick>", COLOR_ERROR)
		"TOPIC":
			if args.size() == 0:
				IRCClient.request_topic(active_target)
			else:
				IRCClient.set_topic(active_target, rest)
		"KICK":
			if args.size() > 0:
				IRCClient.kick(active_target, args[0], " ".join(args.slice(1)))
			else:
				append("Status", "Usage: /kick <nick> [reason]", COLOR_ERROR)
		"ME":
			if active_target != "Status":
				IRCClient.send_ctcp(active_target, "ACTION", rest)
				append_chat(active_target, IRCClient.nickname, "* %s %s" % [IRCClient.nickname, rest])
		"RAW":
			IRCClient.send_raw(rest)
		"CLEAR":
			if active_target in chat_logs:
				chat_logs[active_target].clear()
		"HELP":
			show_help()
		_:
			# Pass unknown commands directly as raw IRC
			IRCClient.send_raw(raw.substr(1))

func show_help() -> void:
	var help : String = """[b]Commands:[/b]
	/connect        — Open connection dialog
	/join #channel  — Join a channel
	/part [#ch]     — Leave channel
	/quit [msg]     — Disconnect
	/msg nick text  — Send private message
	/nick newnick   — Change nickname
	/topic [text]   — Get/set topic
	/kick nick      — Kick from channel
	/me action      — Send action (/me waves)
	/raw text       — Send raw IRC line
	/clear          — Clear chat log
	/help           — This help"""
	append("Status", help, COLOR_SERVER)

func set_status(text: String) -> void:
	status_bar.text = text
