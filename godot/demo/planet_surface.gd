extends MeshInstance3D

## Procedural surface markings for the demo planet (grid + reference points).

const TEX_WIDTH := 512
const TEX_HEIGHT := 256
const GRID_SPACING_DEG := 30.0
const GRID_LINE_WIDTH_DEG := 1.2
const MARKER_RADIUS := 0.014
const SURFACE_OFFSET := 1.004

const _BASE_COLOR := Color(0.12, 0.28, 0.48)
const _GRID_COLOR := Color(0.55, 0.72, 0.88, 0.85)
const _MERIDIAN_COLOR := Color(0.95, 0.85, 0.35, 1.0)
const _EQUATOR_COLOR := Color(0.45, 0.95, 0.55, 1.0)


func _ready() -> void:
	_apply_surface_material()
	_add_reference_markers()


func _apply_surface_material() -> void:
	var material := StandardMaterial3D.new()
	material.albedo_texture = _build_surface_texture()
	material.albedo_color = Color.WHITE
	material.roughness = 0.92
	material.metallic = 0.0
	material_override = material


func _build_surface_texture() -> ImageTexture:
	var image := Image.create(TEX_WIDTH, TEX_HEIGHT, false, Image.FORMAT_RGBA8)
	image.fill(_BASE_COLOR)

	for y in TEX_HEIGHT:
		for x in TEX_WIDTH:
			var u := float(x) / float(TEX_WIDTH)
			var v := float(y) / float(TEX_HEIGHT)
			var lon_deg := u * 360.0 - 180.0
			var lat_deg := 90.0 - v * 180.0
			image.set_pixel(x, y, _pixel_color(lon_deg, lat_deg))

	var texture := ImageTexture.create_from_image(image)
	return texture


func _pixel_color(lon_deg: float, lat_deg: float) -> Color:
	var lon_wrapped := fposmod(lon_deg + 180.0, 360.0) - 180.0

	if _is_near_angle(lon_wrapped, 0.0, GRID_LINE_WIDTH_DEG * 1.4):
		return _MERIDIAN_COLOR
	if _is_near_angle(lat_deg, 0.0, GRID_LINE_WIDTH_DEG * 1.2):
		return _EQUATOR_COLOR
	if _is_grid_line(lon_wrapped, GRID_SPACING_DEG) or _is_grid_line(lat_deg, GRID_SPACING_DEG):
		return _GRID_COLOR

	return _BASE_COLOR


func _is_grid_line(angle_deg: float, spacing_deg: float) -> bool:
	return _is_near_multiple(angle_deg, spacing_deg, GRID_LINE_WIDTH_DEG)


func _is_near_multiple(angle_deg: float, spacing_deg: float, width_deg: float) -> bool:
	var half_spacing := spacing_deg * 0.5
	var remainder := fposmod(angle_deg + half_spacing, spacing_deg) - half_spacing
	return absf(remainder) <= width_deg * 0.5


func _is_near_angle(angle_deg: float, target_deg: float, width_deg: float) -> bool:
	return absf(angle_deg - target_deg) <= width_deg * 0.5


func _add_reference_markers() -> void:
	var marker_mesh := _create_marker_mesh()
	var multimesh := MultiMesh.new()
	multimesh.transform_format = MultiMesh.TRANSFORM_3D
	multimesh.use_colors = true
	multimesh.mesh = marker_mesh

	var marker_specs: Array[Dictionary] = [
		{"lat": 0.0, "lon": 0.0, "color": Color(1.0, 0.25, 0.2)},
		{"lat": 0.0, "lon": 90.0, "color": Color(0.95, 0.95, 0.95)},
		{"lat": 0.0, "lon": -90.0, "color": Color(0.95, 0.95, 0.95)},
		{"lat": 0.0, "lon": 180.0, "color": Color(0.95, 0.95, 0.95)},
		{"lat": 45.0, "lon": 0.0, "color": Color(1.0, 0.75, 0.2)},
		{"lat": -45.0, "lon": 0.0, "color": Color(1.0, 0.75, 0.2)},
		{"lat": 90.0, "lon": 0.0, "color": Color(0.85, 0.4, 1.0)},
		{"lat": -90.0, "lon": 0.0, "color": Color(0.85, 0.4, 1.0)},
	]

	multimesh.instance_count = marker_specs.size()
	for index in marker_specs.size():
		var spec: Dictionary = marker_specs[index]
		var position := _surface_point(spec["lat"], spec["lon"], SURFACE_OFFSET)
		multimesh.set_instance_transform(index, Transform3D(Basis.IDENTITY, position))
		multimesh.set_instance_color(index, spec["color"])

	var markers := MultiMeshInstance3D.new()
	markers.name = "ReferenceMarkers"
	markers.multimesh = multimesh
	markers.material_override = _create_marker_material()
	add_child(markers)


func _create_marker_mesh() -> SphereMesh:
	var mesh := SphereMesh.new()
	mesh.radius = MARKER_RADIUS
	mesh.height = MARKER_RADIUS * 2.0
	mesh.radial_segments = 8
	mesh.rings = 4
	return mesh


func _create_marker_material() -> StandardMaterial3D:
	var material := StandardMaterial3D.new()
	material.vertex_color_use_as_albedo = true
	material.emission_enabled = true
	material.emission_energy_multiplier = 0.35
	material.roughness = 0.4
	return material


func _surface_point(lat_deg: float, lon_deg: float, radius: float) -> Vector3:
	var lat := deg_to_rad(lat_deg)
	var lon := deg_to_rad(lon_deg)
	var cos_lat := cos(lat)
	return Vector3(cos_lat * cos(lon), sin(lat), cos_lat * sin(lon)) * radius
