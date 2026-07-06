class_name VisibleCapMesh
extends MeshInstance3D

const _SURFACE_OFFSET := 1.015
const _RING_COUNT := 32
const _SEGMENT_COUNT := 64


func _ready() -> void:
	_ensure_material()
	cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF


func _ensure_material() -> void:
	if material_override != null:
		return
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.blend_mode = BaseMaterial3D.BLEND_MODE_MIX
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mat.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_ALWAYS
	mat.albedo_color = Color(1.0, 0.82, 0.15, 0.82)
	mat.emission_enabled = true
	mat.emission = Color(1.0, 0.65, 0.1)
	mat.emission_energy_multiplier = 1.6
	mat.render_priority = 10
	material_override = mat


func update_from_observer(
	observer_position: Vector3,
	planet_radius: float,
	horizon_half_angle: float,
) -> void:
	if observer_position.length_squared() <= 0.0 or horizon_half_angle <= 0.0:
		mesh = null
		return

	var center_dir := observer_position.normalized()
	var tangent := center_dir.cross(Vector3.UP)
	if tangent.length_squared() < 0.01:
		tangent = center_dir.cross(Vector3.RIGHT)
	tangent = tangent.normalized()
	var bitangent := center_dir.cross(tangent).normalized()

	var render_radius := planet_radius * _SURFACE_OFFSET
	var rho := minf(horizon_half_angle, PI * 0.49)
	var vertices: PackedVector3Array = []
	var normals: PackedVector3Array = []
	var indices: PackedInt32Array = []

	# Single vertex at the sub-satellite point (cap center).
	vertices.append(center_dir * render_radius)
	normals.append(center_dir)

	for ring in range(1, _RING_COUNT + 1):
		var theta := rho * float(ring) / float(_RING_COUNT)
		var sin_theta := sin(theta)
		var cos_theta := cos(theta)
		for seg in _SEGMENT_COUNT:
			var phi := TAU * float(seg) / float(_SEGMENT_COUNT)
			var direction := (
				center_dir * cos_theta
				+ tangent * (sin_theta * cos(phi))
				+ bitangent * (sin_theta * sin(phi))
			)
			vertices.append(direction * render_radius)
			normals.append(direction)

	# Fan from cap center to the first ring.
	for seg in _SEGMENT_COUNT:
		var next_seg := (seg + 1) % _SEGMENT_COUNT
		indices.append_array([0, 1 + seg, 1 + next_seg])

	# Quads between remaining rings.
	for ring in range(1, _RING_COUNT):
		var ring_base := 1 + (ring - 1) * _SEGMENT_COUNT
		var next_ring_base := 1 + ring * _SEGMENT_COUNT
		for seg in _SEGMENT_COUNT:
			var next_seg := (seg + 1) % _SEGMENT_COUNT
			var i0 := ring_base + seg
			var i1 := ring_base + next_seg
			var i2 := next_ring_base + seg
			var i3 := next_ring_base + next_seg
			indices.append_array([i0, i2, i1, i1, i2, i3])

	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices

	var array_mesh := ArrayMesh.new()
	array_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh = array_mesh
	_ensure_material()
