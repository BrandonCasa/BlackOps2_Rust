"""Decode OAT's BO2 version-19 animation exchange using its retained loader contract."""
import math
import struct

import numpy as np


class Reader:
    def __init__(self, raw):
        self.raw, self.offset = raw, 0

    def take(self, size):
        result = self.raw[self.offset:self.offset + size]
        if len(result) != size:
            raise ValueError("truncated native animation")
        self.offset += size
        return result

    def unpack(self, layout):
        return struct.unpack("<" + layout, self.take(struct.calcsize("<" + layout)))

    def string(self):
        end = self.raw.find(b"\0", self.offset)
        if end < 0:
            raise ValueError("unterminated animation bone/notification name")
        result = self.take(end - self.offset).decode("utf-8")
        self.take(1)
        return result


def decode(raw):
    reader = Reader(raw)
    version, stored_frames, bone_count, flags, asset_type, rate = reader.unpack("HHHBBH")
    if version != 19 or rate == 0:
        raise ValueError("unsupported native animation header")
    looped = bool(flags & 1)
    loop_frames = stored_frames + 1 if looped else stored_frames
    if loop_frames == 0:
        raise ValueError("empty native animation timeline")
    last_frame = loop_frames - 1
    duration = max(last_frame, 1) / rate
    if flags & 8 and not flags & 128:
        reader.unpack("f")  # T5-compatible streamable priming duration.

    def indices(count):
        if count >= loop_frames:
            return np.arange(count, dtype=float) / rate
        layout = "B" if last_frame < 256 else "H"
        return np.asarray(reader.unpack(layout * count), dtype=float) / rate

    def rotations(half, flip=False):
        count, = reader.unpack("H")
        if count == 0:
            return np.array([0., duration]), np.array([[0., 0., 0., 1.]] * 2)
        times = np.array([0.]) if count == 1 else indices(count)
        stored = np.array(reader.unpack("h" * count * (1 if half else 3)), dtype=np.float64).reshape(count, -1)
        omitted = np.sqrt(np.maximum(0, 32767 ** 2 - np.square(stored).sum(axis=1)))
        values = np.zeros((count, 4), dtype=np.float64)
        if half:
            values[:, 2] = stored[:, 0]
        else:
            values[:, :3] = stored
        values[:, 3] = omitted
        values /= np.linalg.norm(values, axis=1, keepdims=True)
        if flip:
            values[0] *= -1
        for i in range(1, count):
            if np.dot(values[i - 1], values[i]) < 0:
                values[i] *= -1
        if count == 1:
            times, values = np.array([0., duration]), np.repeat(values, 2, axis=0)
        return times, values

    def translations():
        count, = reader.unpack("H")
        if count == 0:
            return np.array([0., duration]), np.zeros((2, 3))
        if count == 1:
            return np.array([0., duration]), np.array([reader.unpack("fff")] * 2)
        times = indices(count)
        small, = reader.unpack("B")
        if small not in (0, 1):
            raise ValueError("invalid native translation encoding")
        minimum = np.array(reader.unpack("fff"))
        size = np.array(reader.unpack("fff")) / (255 if small else 65535)
        encoded = np.array(reader.unpack(("B" if small else "H") * count * 3)).reshape(count, 3)
        return times, minimum + encoded * size

    delta = None
    if flags & 2 or (flags & 128 and flags & 4):
        # Locomotion delta is retained separately; fixed-step gameplay owns world motion.
        delta = {"rotation": rotations(not (flags & 128 and flags & 4)), "translation": translations()}
    mask_size = math.ceil(bone_count / 8)
    flip, half = reader.take(mask_size), reader.take(mask_size)
    names = [reader.string() for _ in range(bone_count)]
    if len(set(names)) != len(names):
        raise ValueError("duplicate native animation bone names")
    tracks = {}
    for i, name in enumerate(names):
        tracks[name] = {"rotation": rotations(bool(half[i // 8] & (1 << (i % 8))), bool(flip[i // 8] & (1 << (i % 8)))),
                        "translation": translations()}
    notes = []
    count, = reader.unpack("B")
    for _ in range(count):
        name = reader.string()
        frame, = reader.unpack("H")
        if frame > last_frame:
            raise ValueError("native animation notification outside timeline")
        notes.append({"seconds": frame / rate, "event": name})
    notes.append({"seconds": duration, "event": "end"})
    if reader.offset != len(raw):
        raise ValueError("unconsumed native animation bytes")
    for track in tracks.values():
        for times, values in track.values():
            if not np.isfinite(values).all() or not np.isfinite(times).all() or not (np.diff(times) > 0).all():
                raise ValueError("invalid native animation samples")
    return {"tracks": tracks, "duration": duration, "looped": looped,
            "asset_type": asset_type, "notifications": sorted(notes, key=lambda n: n["seconds"]), "delta": delta}
