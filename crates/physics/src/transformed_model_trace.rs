//! Supplied-pose classic model adapter; scene ownership and family dispatch belong
//! to the caller. No angle/trigonometry or selector defaults are introduced.
use crate::{
    moving_brush_trace::{TraceError, TraceResult},
    trace_shape::checked,
};
use bevy::math::Vec3;
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub enum ModelTransform {
    Translation { origin: Vec3 },
    Matrix { origin: Vec3, rows: [[f32; 3]; 3] },
}

/// Finite transformed endpoints and unrotated centered hull bounds. Selector and
/// mask retain their supplied values; this adapter gives them no family meaning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalModelRequest {
    pub start: Vec3,
    pub end: Vec3,
    pub mins: Vec3,
    pub maxs: Vec3,
    pub selector: u32,
    pub mask: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformedModelError<E> {
    Trace(TraceError),
    Query(E),
}
impl<E> From<TraceError> for TransformedModelError<E> {
    fn from(error: TraceError) -> Self {
        Self::Trace(error)
    }
}
impl<E: fmt::Display> fmt::Display for TransformedModelError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Trace(error) => write!(f, "transformed model trace failed: {error}"),
            Self::Query(error) => write!(f, "transformed model query failed: {error}"),
        }
    }
}
impl<E: std::error::Error + 'static> std::error::Error for TransformedModelError<E> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Trace(error) => Some(error),
            Self::Query(error) => Some(error),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TransformedModelRequest {
    start: Vec3,
    end: Vec3,
    mins: Vec3,
    maxs: Vec3,
    selector: u32,
    mask: u32,
    transform: ModelTransform,
}
impl TransformedModelRequest {
    pub fn new(
        start: Vec3,
        end: Vec3,
        mins: Vec3,
        maxs: Vec3,
        selector: u32,
        mask: u32,
        transform: ModelTransform,
    ) -> Result<Self, TraceError> {
        let origin = match transform {
            ModelTransform::Translation { origin } | ModelTransform::Matrix { origin, .. } => {
                origin
            }
        };
        if ![start, end, mins, maxs, origin]
            .into_iter()
            .all(Vec3::is_finite)
        {
            return Err(TraceError::NonFiniteInput);
        }
        if mins.cmpgt(maxs).any() {
            return Err(TraceError::InvalidExtents);
        }
        if let ModelTransform::Matrix { rows, .. } = transform
            && !rows.into_iter().flatten().all(f32::is_finite)
        {
            return Err(TraceError::NonFiniteInput);
        }
        Ok(Self {
            start,
            end,
            mins,
            maxs,
            selector,
            mask,
            transform,
        })
    }

    /// Stage all authoritative writes, including callback writes and normal
    /// promotion. Errors roll back this result; caller-owned callback side effects
    /// are outside the transaction. Stationary endpoints are passed unchanged.
    pub fn trace_with<T, E>(
        &self,
        incoming: &mut TraceResult,
        query: impl FnOnce(LocalModelRequest, &mut TraceResult) -> Result<T, E>,
    ) -> Result<T, TransformedModelError<E>> {
        validate_result(incoming)?;
        let local = self.local()?;
        let mut staged = *incoming;
        let output = query(local, &mut staged).map_err(TransformedModelError::Query)?;
        validate_result(&staged)?;
        if let ModelTransform::Matrix { rows, .. } = self.transform
            && incoming.fraction > staged.fraction
        {
            let n = staged.normal.to_array();
            let mut world = [0.0; 3];
            for (column, value) in world.iter_mut().enumerate() {
                let y = checked(n[1] * rows[1][column])?;
                let x = checked(n[0] * rows[0][column])?;
                let z = checked(n[2] * rows[2][column])?;
                *value = checked(checked(y + x)? + z)?;
            }
            staged.normal = Vec3::from_array(world);
        }
        *incoming = staged;
        Ok(output)
    }

    fn local(&self) -> Result<LocalModelRequest, TraceError> {
        let origin = match self.transform {
            ModelTransform::Translation { origin } | ModelTransform::Matrix { origin, .. } => {
                origin
            }
        };
        let mut start = self.start.to_array();
        let mut end = self.end.to_array();
        let mut mins = self.mins.to_array();
        let mut maxs = self.maxs.to_array();
        for axis in 0..3 {
            let center = checked(checked(maxs[axis] + mins[axis])? * 0.5)?;
            mins[axis] = checked(mins[axis] - center)?;
            maxs[axis] = checked(maxs[axis] - center)?;
            start[axis] = checked(checked(start[axis] + center)? - origin[axis])?;
            end[axis] = checked(checked(end[axis] + center)? - origin[axis])?;
        }
        if let ModelTransform::Matrix { rows, .. } = self.transform {
            start = row_transform(start, rows)?;
            end = row_transform(end, rows)?;
        }
        Ok(LocalModelRequest {
            start: Vec3::from_array(start),
            end: Vec3::from_array(end),
            mins: Vec3::from_array(mins),
            maxs: Vec3::from_array(maxs),
            selector: self.selector,
            mask: self.mask,
        })
    }
}

fn validate_result(result: &TraceResult) -> Result<(), TraceError> {
    if !result.normal.is_finite() || !result.fraction.is_finite() {
        return Err(TraceError::NonFiniteInput);
    }
    if !(0.0..=1.0).contains(&result.fraction) {
        return Err(TraceError::InvalidFraction);
    }
    Ok(())
}

fn row_transform(point: [f32; 3], rows: [[f32; 3]; 3]) -> Result<[f32; 3], TraceError> {
    let mut local = [0.0; 3];
    for (index, (row, value)) in rows.into_iter().zip(local.iter_mut()).enumerate() {
        let x = checked(point[0] * row[0])?;
        let y = checked(point[1] * row[1])?;
        let z = checked(point[2] * row[2])?;
        // Preserve the distinct native component-one operand order.
        let (first, second) = if index == 1 { (y, x) } else { (x, y) };
        let xy = checked(first + second)?;
        *value = checked(xy + z)?;
    }
    Ok(local)
}
