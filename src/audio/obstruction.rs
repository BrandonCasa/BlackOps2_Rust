use bevy::prelude::*;

pub(super) fn amount(
    listener: Vec3,
    emitter: Vec3,
    trace: impl Fn(Vec3, Vec3) -> Option<f32>,
) -> f32 {
    if trace(listener, emitter).is_none() || listener.distance(emitter) < 0.0254 {
        return 0.;
    }
    let direction = (listener - emitter).normalize_or_zero();
    let side = Vec3::new(direction.z, 0., -direction.x) * (100. * 0.0254);
    let probe = |offset: Vec3| {
        let fraction = trace(listener, listener + offset).unwrap_or(1.);
        if fraction < 0.2 {
            return 1.;
        }
        let mut count = 1;
        let mut clear = 0;
        let mut step = 0.2;
        while step <= fraction {
            count += 1;
            if trace(listener + offset * step, emitter).is_none() {
                clear += 1;
                while step < fraction {
                    count += 1;
                    clear += 1;
                    step += 0.2;
                }
                break;
            }
            step += 0.2;
        }
        1. - clear as f32 / count as f32
    };
    let left = probe(side);
    if left < 1.52879e-05 {
        return 0.;
    }
    left.min(probe(-side))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_clearance_bypasses_side_probes_and_fully_blocked_probes_obstruct() {
        assert_eq!(amount(Vec3::ZERO, Vec3::Z, |_, _| None), 0.);
        assert_eq!(amount(Vec3::ZERO, Vec3::Z, |_, _| Some(0.1)), 1.);
        let opening = amount(Vec3::ZERO, Vec3::Z, |start, end| {
            if start.x.abs() > 0.4 {
                None
            } else if end == Vec3::Z {
                Some(0.5)
            } else {
                None
            }
        });
        assert!(opening > 0. && opening < 1.);
    }
}
