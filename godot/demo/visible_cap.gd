class_name VisibleCapMesh
extends MeshInstance3D

const _SURFACE_OFFSET := 1.015


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


func update_from_cap_mesh(cap_data: Dictionary) -> void:
	var vertices: PackedVector3Array = cap_data.get("cap_vertices", PackedVector3Array())
	var normals: PackedVector3Array = cap_data.get("cap_normals", PackedVector3Array())
	var indices: PackedInt32Array = cap_data.get("cap_indices", PackedInt32Array())
	if vertices.is_empty() or indices.is_empty():
		mesh = null
		return

	var arrays: Array = []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = vertices
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_INDEX] = indices

	var array_mesh := ArrayMesh.new()
	array_mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	mesh = array_mesh
	_ensure_material()
