//! Canonical runtime basis: Bevy +Y up, with original Z-up data adapted at boundaries.
//!
//! This is project policy, not proof of original camera, units or angle conventions.
//! The proper rotation preserves handedness, lengths, winding and raw unit scale.

/// Convert a source position or vector: (x,y,z) -> (x,z,-y).
/// Normals use the same rotation; UVs and triangle indices are unchanged.
pub const fn source_to_bevy([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, z, -y]
}

/// Convert a Bevy position or vector back to source space: (x,y,z) -> (x,-z,y).
pub const fn bevy_to_source([x, y, z]: [f32; 3]) -> [f32; 3] {
    [x, -z, y]
}

/// Adapt an axis-aligned source bound, exchanging endpoints on the reflected axis.
pub const fn source_bounds_to_bevy([lower, upper]: [[f32; 3]; 2]) -> [[f32; 3]; 2] {
    [
        [lower[0], lower[2], -upper[1]],
        [upper[0], upper[2], -lower[1]],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3;

    #[test]
    fn basis_is_y_up_proper_rotation_and_exactly_invertible() {
        let x = Vec3::from_array(source_to_bevy([1.0, 0.0, 0.0]));
        let y = Vec3::from_array(source_to_bevy([0.0, 1.0, 0.0]));
        let z = Vec3::from_array(source_to_bevy([0.0, 0.0, 1.0]));
        assert_eq!(z, Vec3::Y);
        assert_eq!(x.cross(y), z);
        for point in [[2.0, -3.0, 7.0], [-0.0, 0.0, -0.0], [f32::MAX, -1.0, 0.25]] {
            assert_eq!(
                bevy_to_source(source_to_bevy(point)).map(f32::to_bits),
                point.map(f32::to_bits)
            );
        }
        let a = Vec3::new(3.0, -4.0, 12.0);
        let b = Vec3::new(1.0, 2.0, -2.0);
        let convert = |v: Vec3| Vec3::from_array(source_to_bevy(v.to_array()));
        assert_eq!(convert(a).length_squared(), a.length_squared());
        assert_eq!(convert(a).dot(convert(b)), a.dot(b));
        assert_eq!(convert(a.cross(b)), convert(a).cross(convert(b)));
    }

    #[test]
    fn bounds_exchange_reflected_endpoints_and_contain_converted_corners() {
        let bounds = [[-2.0, -7.0, 3.0], [5.0, 11.0, 13.0]];
        let converted = source_bounds_to_bevy(bounds);
        assert_eq!(converted, [[-2.0, 3.0, -11.0], [5.0, 13.0, 7.0]]);
        for x in [bounds[0][0], bounds[1][0]] {
            for y in [bounds[0][1], bounds[1][1]] {
                for z in [bounds[0][2], bounds[1][2]] {
                    let p = source_to_bevy([x, y, z]);
                    assert!(
                        (0..3)
                            .all(|axis| p[axis] >= converted[0][axis]
                                && p[axis] <= converted[1][axis])
                    );
                }
            }
        }
    }
}
