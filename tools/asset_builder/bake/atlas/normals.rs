use super::super::*;

pub(in crate::bake) fn constant_frame(surface: &Surface, first: usize) -> bool {
    (1..3).all(|i| {
        surface.normals[first + i] == surface.normals[first]
            && surface.tangents[first + i] == surface.tangents[first]
    })
}

pub(in crate::bake) fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.into_iter().zip(b).map(|(a, b)| a * b).sum()
}

pub(in crate::bake) fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(in crate::bake) fn unit(value: [f64; 3]) -> Result<[f64; 3]> {
    let length = dot(value, value).sqrt();
    if !length.is_finite() || length <= 1e-12 {
        return Err("Degenerate layered normal frame".into());
    }
    Ok(value.map(|v| v / length))
}

pub(in crate::bake) struct NormalFrame {
    pub(in crate::bake) normals: [[f64; 3]; 3],
    pub(in crate::bake) tangents: [[f64; 3]; 3],
    pub(in crate::bake) bitangents: [[f64; 3]; 3],
    pub(in crate::bake) sign: f64,
}

impl NormalFrame {
    pub(in crate::bake) fn read(surface: &Surface, first: usize) -> Result<Self> {
        let normals = std::array::from_fn(|i| surface.normals[first + i].map(f64::from));
        let tangents = std::array::from_fn(|i| {
            std::array::from_fn(|axis| surface.tangents[first + i][axis] as f64)
        });
        let sign = surface.tangents[first][3] as f64;
        if (0..3).any(|i| !matches!(surface.tangents[first + i][3], -1. | 1.)) {
            return Err(format!(
                "Layered surface {} triangle {} has invalid tangent handedness",
                surface.name,
                first / 3
            )
            .into());
        }
        let bitangents = std::array::from_fn(|i| {
            cross(normals[i], tangents[i]).map(|v| v * surface.tangents[first + i][3] as f64)
        });
        Ok(Self {
            normals,
            tangents,
            bitangents,
            sign,
        })
    }

    pub(in crate::bake) fn convert(
        &self,
        weights: [f64; 3],
        normal: [f32; 4],
    ) -> Result<([f32; 4], [f64; 2])> {
        let interpolate = |values: [[f64; 3]; 3]| {
            std::array::from_fn(|axis| (0..3).map(|i| values[i][axis] * weights[i]).sum::<f64>())
        };
        let n = interpolate(self.normals);
        let t = interpolate(self.tangents);
        let b = interpolate(self.bitangents);
        let source = unit(std::array::from_fn(|axis| {
            t[axis] * normal[0] as f64 + b[axis] * normal[1] as f64 + n[axis] * normal[2] as f64
        }))?;
        let destination_n = unit(n)?;
        let destination_t = unit(t)?;
        let destination_b = cross(destination_n, destination_t).map(|v| v * self.sign);
        let inverse_rows = [
            cross(destination_b, destination_n),
            cross(destination_n, destination_t),
            cross(destination_t, destination_b),
        ];
        let determinant = dot(destination_t, inverse_rows[0]);
        if !determinant.is_finite() || determinant.abs() <= 1e-12 {
            return Err("Singular layered destination normal frame".into());
        }
        let converted = unit(inverse_rows.map(|row| dot(source, row) / determinant))?;
        if converted[2] <= 0. {
            return Err(
                "Layered destination normal exceeds the two-channel positive-Z hemisphere".into(),
            );
        }
        let converted = converted.map(|v| v as f32);
        let raster_b = cross(n, t).map(|v| v * self.sign);
        let direction = |t: [f64; 3], b: [f64; 3], n: [f64; 3]| {
            unit(std::array::from_fn(|axis| {
                t[axis] * converted[0] as f64
                    + b[axis] * converted[1] as f64
                    + n[axis] * converted[2] as f64
            }))
        };
        let angle = |value: [f64; 3]| {
            dot(cross(source, value), cross(source, value))
                .sqrt()
                .atan2(dot(source, value))
                .to_degrees()
        };
        let solari = direction(destination_t, destination_b, destination_n)?;
        let raster = direction(t, raster_b, n)?;
        Ok((
            [converted[0], converted[1], converted[2], 1.],
            [angle(solari), angle(raster)],
        ))
    }
}
