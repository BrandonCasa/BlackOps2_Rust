"""Reconstruct qualified broken surface frames from geometry and authored UVs.

Only material recipes explicitly requesting reconstruction are changed. Source
normals identify smoothing splits; source UVs retain texture seams. Positions,
indices, UVs, materials and placements remain byte-identical.
"""
from collections import defaultdict
import json
import os
from pathlib import Path
import tempfile

import numpy as np


def reconstruct(positions, normals, uvs, tangents, indices):
    triangles = indices.reshape(-1, 3)
    points = positions[triangles].astype(np.float64)
    e1, e2 = points[:, 1] - points[:, 0], points[:, 2] - points[:, 0]
    face = np.cross(e1, e2)
    areas = np.linalg.norm(face, axis=1)
    valid = areas > 1e-12
    face[valid] /= areas[valid, None]
    face[~valid] = 0
    normal_sums, tangent_sums, bitangent_sums = defaultdict(lambda: np.zeros(3)), defaultdict(lambda: np.zeros(3)), defaultdict(lambda: np.zeros(3))
    # Preserve the original smoothing groups rather than welding every touching
    # face of a fractured chunk. UV seams split tangents, but not normals.
    normal_keys = [(tuple(p), tuple(n)) for p, n in zip(positions, normals)]
    tangent_keys = [(k, tuple(uv), float(t[3])) for k, uv, t in zip(normal_keys, uvs, tangents)]
    duv1 = uvs[triangles[:, 1]] - uvs[triangles[:, 0]]
    duv2 = uvs[triangles[:, 2]] - uvs[triangles[:, 0]]
    determinants = duv1[:, 0] * duv2[:, 1] - duv1[:, 1] * duv2[:, 0]
    for ti, ids in enumerate(triangles):
        if not valid[ti]:
            continue
        tangent = bitangent = None
        if abs(determinants[ti]) > 1e-12:
            tangent = (e1[ti] * duv2[ti, 1] - e2[ti] * duv1[ti, 1]) / determinants[ti]
            bitangent = (e2[ti] * duv1[ti, 0] - e1[ti] * duv2[ti, 0]) / determinants[ti]
        for corner, index in enumerate(ids):
            a = points[ti, (corner + 1) % 3] - points[ti, corner]
            b = points[ti, (corner + 2) % 3] - points[ti, corner]
            angle = np.arctan2(np.linalg.norm(np.cross(a, b)), np.dot(a, b))
            normal_sums[normal_keys[index]] += face[ti] * angle
            if tangent is not None:
                tangent_sums[tangent_keys[index]] += tangent * angle
                bitangent_sums[tangent_keys[index]] += bitangent * angle
    result_n, result_t = normals.copy(), np.zeros((len(positions), 4), np.float32)
    for index, key in enumerate(normal_keys):
        normal = normal_sums[key]
        length = np.linalg.norm(normal)
        if length > 1e-12:
            result_n[index] = normal / length
        normal = result_n[index]
        tangent = tangent_sums[tangent_keys[index]]
        tangent = tangent - normal * np.dot(normal, tangent)
        length = np.linalg.norm(tangent)
        if length < 1e-12:
            axis = np.eye(3)[np.argmin(abs(normal))]
            tangent = np.cross(axis, normal)
            length = np.linalg.norm(tangent)
        tangent /= length
        handedness = -1 if np.dot(np.cross(normal, tangent), bitangent_sums[tangent_keys[index]]) < 0 else 1
        result_t[index] = [*tangent, handedness]
    return result_n.astype('<f4'), result_t.astype('<f4')


def repair_document(pack, path, catalog):
    document = json.loads(path.read_text())
    requested = {i for i, m in enumerate(document.get('materials', []))
                 if catalog['materials'].get(m['name'], {}).get('reconstruct_surface_frame')}
    if not requested:
        return []
    buffers = [(path.parent / b['uri']).read_bytes() for b in document['buffers']]
    changed_buffers, receipt = {}, []

    def read(index):
        a = document['accessors'][index]; v = document['bufferViews'][a['bufferView']]
        dtype = {5126: '<f4', 5125: '<u4', 5123: '<u2', 5121: 'u1'}[a['componentType']]
        width = {'SCALAR': 1, 'VEC2': 2, 'VEC3': 3, 'VEC4': 4}[a['type']]
        offset = v.get('byteOffset', 0) + a.get('byteOffset', 0)
        stride = v.get('byteStride', np.dtype(dtype).itemsize * width)
        return np.ndarray((a['count'], width), dtype, buffer=buffers[v['buffer']], offset=offset,
                          strides=(stride, np.dtype(dtype).itemsize)).copy()

    def write(index, values):
        a = document['accessors'][index]; v = document['bufferViews'][a['bufferView']]
        if a['componentType'] != 5126 or a['count'] != len(values):
            raise ValueError('surface frame accessor contract changed')
        data = changed_buffers.setdefault(v['buffer'], bytearray(buffers[v['buffer']]))
        offset = v.get('byteOffset', 0) + a.get('byteOffset', 0)
        stride = v.get('byteStride', values.shape[1] * 4)
        for index, value in enumerate(values):
            data[offset + index * stride:offset + index * stride + values.shape[1] * 4] = value.tobytes()
        a['min'], a['max'] = values.min(axis=0).tolist(), values.max(axis=0).tolist()

    for mesh_index, mesh in enumerate(document.get('meshes', [])):
        for primitive in mesh['primitives']:
            if primitive.get('material') not in requested or primitive.get('mode', 4) != 4:
                continue
            attrs = primitive['attributes']
            positions, normals, uvs = [read(attrs[k]) for k in ('POSITION', 'NORMAL', 'TEXCOORD_0')]
            indices = read(primitive['indices']).flatten()
            fixed_n, fixed_t = reconstruct(positions, normals, uvs, read(attrs['TANGENT']), indices)
            write(attrs['NORMAL'], fixed_n)
            write(attrs['TANGENT'], fixed_t)
            material = document['materials'][primitive['material']]
            material['extras']['surface']['provenance']['surface_frame'] = 'angle-weighted geometry normals and UV tangents; retained smoothing and UV splits'
            receipt.append({'mesh': mesh_index, 'material': material['name'], 'vertices': len(positions),
                            'changed_normals': int(np.count_nonzero(np.linalg.norm(fixed_n - normals, axis=1) > .001))})
    # Replace entries, never mutate an original or an immutable pack hardlink.
    for index, data in changed_buffers.items():
        replace(path.parent / document['buffers'][index]['uri'], data)
    if receipt:
        replace(path, (json.dumps(document, separators=(',', ':')) + '\n').encode())
    return receipt


def replace(path, data):
    descriptor, temporary = tempfile.mkstemp(prefix='.' + path.name + '.', dir=path.parent)
    try:
        with os.fdopen(descriptor, 'wb') as stream:
            stream.write(data)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)
