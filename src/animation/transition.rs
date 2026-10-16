/// T6 XAnim goal weights (native 00789880 / 0078a880).
/// The curve is linear in remaining time; a reversal preserves the current weight.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct GoalWeight {
    pub weight: f32,
    pub target: f32,
    remaining: f32,
}

impl GoalWeight {
    pub fn set(&mut self, target: f32, seconds: f32) {
        if seconds == 0. {
            self.weight = target;
            self.remaining = 0.;
        } else {
            if self.weight == 0. {
                self.weight = target * 0.001;
            }
            let difference = self.weight - target;
            self.remaining = if difference == 0. || (self.target - target) * difference > 0. {
                seconds
            } else {
                self.remaining.min(seconds)
            };
            if target != 0. {
                let distance = if target < self.weight {
                    difference / self.weight
                } else {
                    -difference / target
                };
                self.remaining = self.remaining.max(distance * seconds);
            }
        }
        self.target = target;
    }

    pub fn advance(&mut self, dt: f32) -> f32 {
        if self.remaining <= dt {
            self.weight = self.target;
            self.remaining = 0.;
        } else {
            self.weight += (self.target - self.weight) * dt / self.remaining;
            self.remaining -= dt;
            if self.weight < 1.0000001e-6 {
                self.weight = self.target * 0.001;
            }
        }
        self.weight
    }
}

/// Incoming SetAnim goal time and outgoing ClearAnim goal time are independent.
#[derive(Clone, Copy)]
pub(crate) struct ClipBlend {
    pub incoming: f32,
    pub outgoing: f32,
}
impl ClipBlend {
    pub const CUT: Self = Self::crossfade(0.);
    pub const fn crossfade(seconds: f32) -> Self {
        Self {
            incoming: seconds,
            outgoing: seconds,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_goal_is_linear_and_independent_of_step_size() {
        let mut whole = GoalWeight::default();
        whole.set(1., 0.2);
        let mut split = whole;
        let half = whole.advance(0.1);
        split.advance(0.04);
        assert!((split.advance(0.06) - half).abs() < 0.000001);
        assert!((half - 0.5005).abs() < 0.000001);
        assert_eq!(whole.advance(0.1), 1.);
    }

    #[test]
    fn reversal_and_pause_preserve_the_current_weight() {
        let mut goal = GoalWeight::default();
        goal.set(1., 0.2);
        let current = goal.advance(0.05);
        goal.set(0., 0.2);
        assert_eq!(goal.advance(0.), current);
        assert!((goal.advance(0.1) - current * 0.5).abs() < 0.000001);
        goal.set(1., 0.2);
        assert!(goal.advance(0.05) > current * 0.5);
    }

    #[test]
    fn zero_goal_time_is_an_immediate_cut() {
        let mut goal = GoalWeight::default();
        goal.set(1., 0.2);
        goal.advance(0.05);
        goal.set(0., 0.);
        assert_eq!(goal.advance(0.), 0.);
        goal.set(1., 0.);
        assert_eq!(goal.advance(0.), 1.);
    }
}
