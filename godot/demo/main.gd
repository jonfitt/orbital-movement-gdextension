extends Node3D

const ORBIT_OPTIONS: Array[Dictionary] = [
	{
		"id": OrbitalSimulation.ORBIT_CIRCULAR_EQUATORIAL,
		"label": "Circular equatorial",
	},
	{
		"id": OrbitalSimulation.ORBIT_CIRCULAR_POLAR,
		"label": "Circular polar",
	},
	{
		"id": OrbitalSimulation.ORBIT_GEOSTATIONARY,
		"label": "Geostationary",
	},
	{
		"id": OrbitalSimulation.ORBIT_LOW_CIRCULAR,
		"label": "Low circular",
	},
	{
		"id": OrbitalSimulation.ORBIT_RETROGRADE_EQUATORIAL,
		"label": "Retrograde equatorial",
	},
	{
		"id": OrbitalSimulation.ORBIT_ECLIPTIC_PROGRADE,
		"label": "Ecliptic prograde",
	},
	{
		"id": OrbitalSimulation.ORBIT_ECLIPTIC_RETROGRADE,
		"label": "Ecliptic retrograde",
	},
	{
		"id": OrbitalSimulation.ORBIT_ELLIPTICAL_EQUATORIAL,
		"label": "Elliptical equatorial",
	},
	{
		"id": OrbitalSimulation.ORBIT_ELLIPTICAL_INCLINED,
		"label": "Elliptical inclined",
	},
	{
		"id": OrbitalSimulation.ORBIT_TUNDRA,
		"label": "Tundra (inclined GEO)",
	},
	{
		"id": OrbitalSimulation.ORBIT_MOLNIYA,
		"label": "Molniya",
	},
	{
		"id": OrbitalSimulation.ORBIT_GRAVEYARD,
		"label": "Graveyard (above GEO)",
	},
]

const _AXIAL_TILT_RAD := 0.41
const _DEFAULT_TRANSFER_THRUST := 0.002
const _MAX_PRACTICAL_BURN_TIME_S := 3600.0
const _SATELLITE_MASS := 1.0
const _TRACK_DISPLAY_OFFSET := 1.014
const _CAP_DISPLAY_OFFSET := 1.015

@onready var _label: Label = $CanvasLayer/Label
@onready var _time_label: Label = $CanvasLayer/TimeScalePanel/TimeLabel
@onready var _time_slider: HSlider = $CanvasLayer/TimeScalePanel/TimeSlider
@onready var _orbit_select: OptionButton = $CanvasLayer/ControlPanel/OrbitPanel/OrbitSelect
@onready var _altitude_edit: LineEdit = $CanvasLayer/ControlPanel/OrbitPanel/AltitudeRow/AltitudeEdit
@onready var _inclination_row: HBoxContainer = $CanvasLayer/ControlPanel/OrbitPanel/InclinationRow
@onready var _inclination_edit: LineEdit = $CanvasLayer/ControlPanel/OrbitPanel/InclinationRow/InclinationEdit
@onready var _apogee_row: HBoxContainer = $CanvasLayer/ControlPanel/OrbitPanel/ApogeeRow
@onready var _apogee_edit: LineEdit = $CanvasLayer/ControlPanel/OrbitPanel/ApogeeRow/ApogeeEdit
@onready var _place_orbit_button: Button = $CanvasLayer/ControlPanel/OrbitPanel/OrbitButtons/PlaceOrbitButton
@onready var _transfer_button: Button = $CanvasLayer/ControlPanel/OrbitPanel/OrbitButtons/TransferButton
@onready var _transfer_thrust_edit: LineEdit = (
	$CanvasLayer/ControlPanel/OrbitPanel/TransferThrustRow/TransferThrustEdit
)
@onready var _transfer_status: Label = $CanvasLayer/ControlPanel/OrbitPanel/TransferStatusRow/TransferStatusLabel
@onready var _transfer_progress: ProgressBar = (
	$CanvasLayer/ControlPanel/OrbitPanel/TransferStatusRow/TransferProgressBar
)
@onready var _thrust_edit: LineEdit = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/ThrustMagnitudeRow/ThrustEdit
@onready var _chk_prograde: CheckBox = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/DirectionGrid/ProgradeCheck
@onready var _chk_retrograde: CheckBox = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/DirectionGrid/RetrogradeCheck
@onready var _chk_left: CheckBox = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/DirectionGrid/LeftCheck
@onready var _chk_right: CheckBox = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/DirectionGrid/RightCheck
@onready var _chk_up: CheckBox = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/DirectionGrid/UpCheck
@onready var _chk_down: CheckBox = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/DirectionGrid/DownCheck
@onready var _thrust_toggle: Button = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/ThrustToggle
@onready var _track_toggle: Button = $CanvasLayer/ControlPanel/OrbitPanel/ThrustPanel/TrackToggle
@onready var _planet: MeshInstance3D = $Planet
@onready var _camera_pivot: Node3D = $Planet/CameraPivot
@onready var _satellite: MeshInstance3D = $Satellite
@onready var _visible_cap: VisibleCapMesh = $VisibleCap
@onready var _ground_track: OrbitalGroundTrackMesh = $GroundTrack
@onready var _sun: DirectionalLight3D = $Sun
@onready var _camera: Camera3D = $Planet/CameraPivot/Camera3D

var _sim: OrbitalSimulation
var _body_id: int = -1
var _sim_spin_angle: float = 0.0
var _planet_view_angle: float = 0.0
var _time_scale: float = 60.0
var _thrust_active: bool = false
var _track_visible: bool = false

var _cam_yaw: float = 0.8
var _cam_pitch: float = 0.35
var _cam_distance: float = 3.5
var _cam_orbitting: bool = false
var _planet_rotating: bool = false


func _fmt_num(value: float, decimals: int = 2) -> String:
	var step := pow(10.0, -decimals)
	return str(snapped(value, step))


func _ready() -> void:
	_sim = OrbitalSimulation.new()
	_sim.create_earth_like_with_obliquity(86_400.0, _AXIAL_TILT_RAD)

	for option in ORBIT_OPTIONS:
		_orbit_select.add_item(option["label"])

	_orbit_select.select(3)
	_altitude_edit.text = "0.05"
	var leo_defaults := _sim.get_orbit_ui_defaults(OrbitalSimulation.ORBIT_LOW_CIRCULAR)
	_inclination_edit.text = str(leo_defaults.get("inclination_deg", 51.6))
	_apogee_edit.text = "0.20"
	_thrust_edit.text = "0.001"
	_transfer_thrust_edit.text = str(_DEFAULT_TRANSFER_THRUST)
	_thrust_toggle.text = "Thrust OFF"
	_track_toggle.text = "Ground track OFF"

	_orbit_select.item_selected.connect(_on_orbit_selected)
	_place_orbit_button.pressed.connect(_on_place_in_orbit_pressed)
	_transfer_button.pressed.connect(_on_transfer_pressed)
	_transfer_thrust_edit.text_changed.connect(_on_transfer_thrust_changed)
	_thrust_toggle.pressed.connect(_on_thrust_toggle_pressed)
	_track_toggle.pressed.connect(_on_track_toggle_pressed)

	_time_slider.min_value = 0.0
	_time_slider.max_value = 4.0
	_time_slider.step = 0.05
	_time_slider.value = log(_time_scale) / log(10.0)
	_time_slider.value_changed.connect(_on_time_slider_changed)

	_on_orbit_selected(_orbit_select.selected)
	_spawn_satellite()
	_update_time_scale_label()
	_update_camera()
	_update_scene()


func _physics_process(delta: float) -> void:
	var sim_delta := delta * _time_scale
	if _thrust_active and _body_id >= 0:
		_apply_manual_thrust_limit()
		var magnitude := _read_thrust_magnitude()
		var flags := _collect_thrust_flags()
		if magnitude > 0.0 and flags != 0:
			_sim.apply_force_from_flags(_body_id, magnitude, flags)
	_sim_spin_angle += sim_delta * _sim.get_angular_rate_rad_s()
	_sim.step(sim_delta)
	_update_scene()


func _planet_display_basis() -> Basis:
	return Basis(Vector3.UP, _planet_view_angle)


func _selected_orbit_id() -> int:
	return ORBIT_OPTIONS[_orbit_select.selected]["id"]


func _gather_orbit_inputs() -> Dictionary:
	var orbit_id := _selected_orbit_id()
	var altitude := _read_altitude()
	var perigee := _read_perigee()
	var apogee := _read_apogee()
	if not OrbitalSimulation.orbit_uses_elliptical_params(orbit_id):
		perigee = altitude
	return {
		"altitude": altitude,
		"perigee": perigee,
		"apogee": apogee,
		"inclination": _read_inclination_rad(),
	}


func _spawn_satellite() -> void:
	var inputs := _gather_orbit_inputs()
	_body_id = _sim.spawn_body_in_orbit(
		_selected_orbit_id(),
		inputs.altitude,
		inputs.perigee,
		inputs.apogee,
		inputs.inclination,
		_SATELLITE_MASS,
	)
	if _body_id >= 0:
		_apply_transfer_thrust_limit()


func _read_altitude() -> float:
	return maxf(_altitude_edit.text.to_float(), 0.0)


func _read_apogee() -> float:
	return maxf(_apogee_edit.text.to_float(), _read_perigee() + 0.01)


func _read_perigee() -> float:
	return maxf(_altitude_edit.text.to_float(), 0.0)


func _read_inclination_rad() -> float:
	return deg_to_rad(clampf(_inclination_edit.text.to_float(), 0.0, 90.0))


func _read_thrust_magnitude() -> float:
	return maxf(_thrust_edit.text.to_float(), 0.0)


func _read_transfer_thrust() -> float:
	return maxf(_transfer_thrust_edit.text.to_float(), 0.0)


func _apply_transfer_thrust_limit() -> void:
	if _body_id >= 0:
		_sim.set_max_thrust(_body_id, _read_transfer_thrust())


func _apply_manual_thrust_limit() -> void:
	if _body_id >= 0:
		_sim.set_max_thrust(_body_id, maxf(_read_transfer_thrust(), _read_thrust_magnitude()))


func _collect_thrust_flags() -> int:
	var flags := 0
	if _chk_prograde.button_pressed:
		flags |= OrbitalSimulation.THRUST_PROGRADE
	if _chk_retrograde.button_pressed:
		flags |= OrbitalSimulation.THRUST_RETROGRADE
	if _chk_left.button_pressed:
		flags |= OrbitalSimulation.THRUST_LEFT
	if _chk_right.button_pressed:
		flags |= OrbitalSimulation.THRUST_RIGHT
	if _chk_up.button_pressed:
		flags |= OrbitalSimulation.THRUST_UP
	if _chk_down.button_pressed:
		flags |= OrbitalSimulation.THRUST_DOWN
	return flags


func _apply_orbit_ui_defaults() -> void:
	var defaults := _sim.get_orbit_ui_defaults(_selected_orbit_id())
	if defaults.is_empty():
		return
	_altitude_edit.text = _fmt_num(defaults["altitude_earth_radii"], 2)
	_apogee_edit.text = _fmt_num(defaults["apogee_altitude_earth_radii"], 2)
	_inclination_edit.text = _fmt_num(defaults["inclination_deg"], 1)


func _on_orbit_selected(_index: int) -> void:
	var orbit_id := _selected_orbit_id()
	_apogee_row.visible = OrbitalSimulation.orbit_uses_elliptical_params(orbit_id)
	_inclination_row.visible = OrbitalSimulation.orbit_uses_inclination_param(orbit_id)

	if orbit_id == OrbitalSimulation.ORBIT_GEOSTATIONARY:
		_altitude_edit.editable = false
		_altitude_edit.text = _fmt_num(_sim.get_geostationary_altitude(), 2)
		_altitude_edit.tooltip_text = (
			"Computed from planet mass and rotation period (user value ignored)"
		)
	elif orbit_id == OrbitalSimulation.ORBIT_GRAVEYARD:
		_altitude_edit.editable = false
		_altitude_edit.text = _fmt_num(_sim.get_graveyard_altitude(), 2)
		_altitude_edit.tooltip_text = "Supersynchronous altitude above GEO (computed)"
	elif orbit_id == OrbitalSimulation.ORBIT_TUNDRA:
		_altitude_edit.editable = false
		_altitude_edit.text = _fmt_num(_sim.get_geostationary_altitude(), 2)
		_altitude_edit.tooltip_text = "Geostationary altitude with inclined ground track"
		_apply_orbit_ui_defaults()
	elif OrbitalSimulation.orbit_uses_computed_altitude(orbit_id):
		_altitude_edit.editable = false
	else:
		_altitude_edit.editable = true
		_altitude_edit.tooltip_text = (
			"Perigee altitude (R⊕)" if _apogee_row.visible else "Altitude above surface in Earth radii"
		)
		if orbit_id == OrbitalSimulation.ORBIT_MOLNIYA:
			_apply_orbit_ui_defaults()

	_update_transfer_button_state()


func _on_transfer_thrust_changed(_new_text: String) -> void:
	_update_transfer_button_state()


func _assess_transfer() -> Dictionary:
	if _body_id < 0:
		return {}
	var inputs := _gather_orbit_inputs()
	return _sim.assess_transfer_viability(
		_body_id,
		_selected_orbit_id(),
		inputs.altitude,
		inputs.perigee,
		inputs.apogee,
		inputs.inclination,
		_MAX_PRACTICAL_BURN_TIME_S,
		_read_transfer_thrust(),
	)


func _update_transfer_button_state() -> void:
	var report := _assess_transfer()
	if report.is_empty():
		_transfer_button.disabled = true
		_transfer_button.tooltip_text = ""
		return

	var availability: int = report["availability"]
	_transfer_button.disabled = (
		availability == OrbitalSimulation.TRANSFER_VIABILITY_UNAVAILABLE
	)
	match availability:
		OrbitalSimulation.TRANSFER_VIABILITY_IMPRACTICAL:
			_transfer_button.tooltip_text = report.get("reason", "Impractical transfer")
		OrbitalSimulation.TRANSFER_VIABILITY_UNAVAILABLE:
			_transfer_button.tooltip_text = report.get("reason", "Transfer unavailable")
		_:
			_transfer_button.tooltip_text = ""


func _on_place_in_orbit_pressed() -> void:
	_thrust_active = false
	_thrust_toggle.text = "Thrust OFF"
	_clear_transfer_indicator()
	_sim.reset_simulation()
	_sim_spin_angle = 0.0
	_spawn_satellite()
	if _track_visible:
		_refresh_ground_track()
	_update_scene()


func _on_transfer_pressed() -> void:
	if _body_id < 0:
		return

	var report := _assess_transfer()
	if report.get("availability", OrbitalSimulation.TRANSFER_VIABILITY_UNAVAILABLE) == (
		OrbitalSimulation.TRANSFER_VIABILITY_UNAVAILABLE
	):
		_transfer_status.text = "Transfer: %s" % report.get("reason", "unavailable")
		_transfer_progress.value = 0.0
		return

	_apply_transfer_thrust_limit()
	var inputs := _gather_orbit_inputs()
	var started := _sim.begin_transfer_to_orbit(
		_body_id,
		_selected_orbit_id(),
		inputs.altitude,
		inputs.perigee,
		inputs.apogee,
		inputs.inclination,
	)
	if started:
		_update_transfer_indicator()
	else:
		_transfer_status.text = "Transfer: failed (set max_thrust?)"
		_transfer_progress.value = 0.0


func _clear_transfer_indicator() -> void:
	if _body_id >= 0:
		_sim.clear_transfer_burn(_body_id)
	_transfer_status.text = "Transfer: idle"
	_transfer_progress.value = 0.0


func _update_transfer_indicator() -> void:
	if _body_id < 0:
		_transfer_status.text = "Transfer: idle"
		_transfer_progress.value = 0.0
		return

	var status := _sim.get_transfer_burn_status(_body_id)
	var progress := _sim.get_transfer_burn_progress(_body_id)
	_transfer_progress.value = progress * 100.0

	match status:
		OrbitalSimulation.TRANSFER_BURNING:
			_transfer_status.text = (
				"Transfer: burning (%s%% done, dV remaining=%s)"
				% [
					_fmt_num(progress * 100.0, 0),
					_fmt_num(_sim.get_transfer_burn_remaining(_body_id), 4),
				]
			)
		OrbitalSimulation.TRANSFER_FINISHED:
			_transfer_status.text = "Transfer: complete"
		_:
			_transfer_status.text = "Transfer: idle"
			_transfer_progress.value = 0.0


func _on_thrust_toggle_pressed() -> void:
	_thrust_active = not _thrust_active
	_thrust_toggle.text = "Thrust ON" if _thrust_active else "Thrust OFF"
	if _track_visible:
		_refresh_ground_track()


func _on_track_toggle_pressed() -> void:
	_track_visible = not _track_visible
	_track_toggle.text = "Ground track ON" if _track_visible else "Ground track OFF"
	if _track_visible:
		_refresh_ground_track()
	else:
		_ground_track.clear()


func _refresh_ground_track() -> void:
	if _body_id < 0 or not _track_visible:
		return

	var display_radius := _sim.get_planet_radius() * _TRACK_DISPLAY_OFFSET
	var track := _sim.get_orbital_surface_track(_body_id, _sim_spin_angle, 256, display_radius)
	_ground_track.update_from_track_data(track)


func _update_scene() -> void:
	if _body_id < 0:
		return

	var inertial_pos := _sim.get_position(_body_id)
	var display_pos := _sim.get_position_planet_fixed(_body_id, _sim_spin_angle)
	_satellite.position = display_pos

	_planet.basis = _planet_display_basis()

	var planet_radius := _sim.get_planet_radius()
	var cap_radius := planet_radius * _CAP_DISPLAY_OFFSET
	var cap_mesh := _sim.get_visibility_cap_mesh(_body_id, _sim_spin_angle, cap_radius)
	_visible_cap.update_from_cap_mesh(cap_mesh)

	if _track_visible:
		_refresh_ground_track()

	var sun_pos := _sim.get_star_apparent_position(_sim_spin_angle)
	_sun.position = sun_pos.normalized() * 20.0
	_sun.look_at(Vector3.ZERO, Vector3.UP)

	var area := _sim.get_visible_surface_area(_body_id)
	var horizon := _sim.get_horizon_half_angle(_body_id)
	var state := _sim.get_state(_body_id)
	var state_text := (
		"SurfaceContact" if state == OrbitalSimulation.STATE_SURFACE_CONTACT else "Flying"
	)
	_update_transfer_indicator()
	_update_transfer_button_state()
	_label.text = (
		"Orbital demo | sim t=%s s | pos=(%s, %s, %s) | visible area=%s | horizon=%s deg | state=%s\n"
		% [
			_fmt_num(_sim.get_time(), 0),
			_fmt_num(inertial_pos.x),
			_fmt_num(inertial_pos.y),
			_fmt_num(inertial_pos.z),
			_fmt_num(area, 3),
			_fmt_num(rad_to_deg(horizon), 1),
			state_text,
		]
		+ "LMB drag: orbit camera | RMB drag: rotate planet | Wheel: zoom | [/]: time scale"
	)


func _update_camera() -> void:
	var offset := Vector3(
		cos(_cam_pitch) * sin(_cam_yaw),
		sin(_cam_pitch),
		cos(_cam_pitch) * cos(_cam_yaw),
	) * _cam_distance
	_camera_pivot.position = Vector3.ZERO
	_camera.position = offset
	_camera.look_at(Vector3.ZERO, Vector3.UP)


func _set_time_scale(value: float) -> void:
	_time_scale = clampf(value, 0.0, 10_000.0)
	_time_slider.set_value_no_signal(log(maxf(_time_scale, 0.01)) / log(10.0))
	_update_time_scale_label()


func _update_time_scale_label() -> void:
	if _time_scale <= 0.0:
		_time_label.text = "Time scale: paused"
	else:
		_time_label.text = (
			"Time scale: %s x (1 s real = %s s sim)"
			% [_fmt_num(_time_scale, 1), _fmt_num(_time_scale, 0)]
		)


func _on_time_slider_changed(log10_value: float) -> void:
	_set_time_scale(pow(10.0, log10_value))


func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			_cam_orbitting = event.pressed
		elif event.button_index == MOUSE_BUTTON_RIGHT:
			_planet_rotating = event.pressed
		elif event.button_index == MOUSE_BUTTON_WHEEL_UP and event.pressed:
			_cam_distance = maxf(1.5, _cam_distance - 0.25)
			_update_camera()
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and event.pressed:
			_cam_distance = minf(12.0, _cam_distance + 0.25)
			_update_camera()

	if event is InputEventMouseMotion and _cam_orbitting:
		_cam_yaw -= event.relative.x * 0.005
		_cam_pitch = clampf(_cam_pitch - event.relative.y * 0.005, -1.2, 1.2)
		_update_camera()

	if event is InputEventMouseMotion and _planet_rotating:
		_planet_view_angle -= event.relative.x * 0.005
		_update_scene()

	if event is InputEventKey and event.pressed and not event.echo:
		match event.keycode:
			KEY_ESCAPE:
				get_tree().quit()
			KEY_BRACKETLEFT, KEY_MINUS:
				_set_time_scale(_time_scale * 0.5 if _time_scale > 0.0 else 1.0)
			KEY_BRACKETRIGHT, KEY_EQUAL, KEY_KP_ADD:
				_set_time_scale(_time_scale * 2.0 if _time_scale > 0.0 else 1.0)
			KEY_P:
				_set_time_scale(0.0 if _time_scale > 0.0 else 60.0)
