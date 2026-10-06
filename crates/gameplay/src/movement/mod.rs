//! Qualified movement building blocks; this is not a complete movement motor.
//!
//! Timing follows native RVAs 23af10/544b30. Ordinary acceleration follows
//! 53dc10's flag-bit-3-clear scalar SSE path. Profiles, mode eligibility,
//! collision, x87 kernels and camera behavior remain outside this boundary.

pub mod acceleration;
pub mod jump_eligibility;
pub mod posture_timing;
pub mod sprint_budget;
pub mod timing;
