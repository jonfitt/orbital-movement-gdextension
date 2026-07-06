class_name OrbitalGroundTrackMesh
extends Node3D

var _ground_mesh: MeshInstance3D
var _corridor_mesh: MeshInstance3D


func _ready() -> void:
	_ground_mesh = _make_line_mesh(Color(0.2, 0.95, 1.0, 0.95), 12)
	_corridor_mesh = _make_corridor_mesh()
	add_child(_ground_mesh)
	add_child(_corridor_mesh)
	clear()


func _make_line_mesh(color: Color, render_priority: int) -> MeshInstance3D:
	var mesh_instance := MeshInstance3D.new()
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.blend_mode = BaseMaterial3D.BLEND_MODE_MIX
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mat.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_ALWAYS
	mat.albedo_color = color
	mat.emission_enabled = true
	mat.emission = color
	mat.emission_energy_multiplier = 1.2
	mat.render_priority = render_priority
	mesh_instance.material_override = mat
	mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return mesh_instance


func _make_corridor_mesh() -> MeshInstance3D:
	var mesh_instance := MeshInstance3D.new()
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.blend_mode = BaseMaterial3D.BLEND_MODE_MIX
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	mat.depth_draw_mode = BaseMaterial3D.DEPTH_DRAW_ALWAYS
	mat.albedo_color = Color(1.0, 0.9, 0.45, 0.38)
	mat.emission_enabled = true
	mat.emission = Color(1.0, 0.82, 0.28)
	mat.emission_energy_multiplier = 0.9
	mat.render_priority = 9
	mesh_instance.material_override = mat
	mesh_instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	return mesh_instance


func clear() -> void:
	_ground_mesh.mesh = null
	_corridor_mesh.mesh = null


func update_from_track_data(track_data: Dictionary) -> void:
	if track_data.is_empty():
		clear()
		return

	var ground_line: PackedVector3Array = track_data.get(
		"ground_line_vertices",
		PackedVector3Array(),
	)
	var corridor_vertices: PackedVector3Array = track_data.get(
		"corridor_vertices",
		PackedVector3Array(),
	)
	var corridor_normals: PackedVector3Array = track_data.get(
		"corridor_normals",
		PackedVector3Array(),
	)
	var corridor_indices: PackedInt32Array = track_data.get(
		"corridor_indices",
		PackedInt32Array(),
	)

	if ground_line.is_empty() and corridor_vertices.is_empty():
		clear()
		return

	_ground_mesh.mesh = _mesh_from_line_vertices(ground_line)
	_corridor_mesh.mesh = _mesh_from_surface_arrays(
		corridor_vertices,
		corridor_normals,
		corridor_indices,
	)


func _mesh_from_line_vertices(vertices: PackedVector3Array) -> ArrayMesh:
	if vertices.is_empty():
		return null
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_LINE_STRIP, arrays)
	return mesh


func _mesh_from_surface_arrays(
	vertices: PackedVector3Array,
	normals: PackedVector3Array,
	indices: PackedInt32Array,
) -> ArrayMesh:
	if vertices.is_empty() or indices.is_empty():
		return null
	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return mesh
