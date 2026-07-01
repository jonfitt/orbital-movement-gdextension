extends Node3D

@onready var _label: Label = $CanvasLayer/Label


func _ready() -> void:
	var api := OrbitalMovementGdextensionApi.new()
	_label.text = api.greet("Godot demo")


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		if event.keycode == KEY_ESCAPE:
			get_tree().quit()
