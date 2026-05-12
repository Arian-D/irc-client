extends Window

@onready var server_field : LineEdit = $VBox/Grid/ServerField
@onready var port_field : SpinBox = $VBox/Grid/PortField
@onready var nick_field : LineEdit = $VBox/Grid/NickField
@onready var user_field : LineEdit = $VBox/Grid/UserField
@onready var real_field : LineEdit = $VBox/Grid/RealField
@onready var pass_field : LineEdit = $VBox/Grid/PassField
@onready var connect_btn : Button = $VBox/ConnectBtn
@onready var error_label : Label = $VBox/ErrorLabel

func _ready() -> void:
	title = "Connect to IRC Server"
	
	server_field.text  = "irc.libera.chat"
	port_field.value   = 6667
	nick_field.text    = "verychatmard"
	user_field.text    = "verychatmard"
	real_field.text    = "VeryChat Client"
	pass_field.secret  = true
	error_label.text   = ""

	connect_btn.pressed.connect(on_connect_pressed)
	close_requested.connect(hide)

func on_connect_pressed() -> void:
	error_label.text = ""
	
	var server : String = server_field.text.strip_edges()
	var port : int = int(port_field.value)
	var nick : String = nick_field.text.strip_edges()
	var user : String = user_field.text.strip_edges()
	var real : String = real_field.text.strip_edges()
	var passwd : String = pass_field.text

	if server == "":
		error_label.text = "Server address required."
		return
	if nick == "":
		error_label.text = "Nickname required."
		return
	if not nick.is_valid_ascii_identifier():
		error_label.text = "Invalid nickname."
		return

	connect_btn.disabled = true
	connect_btn.text = "Connecting.."

	IRCClient.connect_to_server(server, port, nick, user, real, passwd)
	await get_tree().create_timer(3.0).timeout

	connect_btn.disabled = false
	connect_btn.text = "Connect"
	hide()
