//! The game's Havok world (Havok 7.1.0-r1 as linked into FalloutNV.exe
//! 1.4.0.525), translated piece by piece (B1, docs/PHYSICS.md "Havok's
//! world step"). Names are the Xbox 360 prototype's (Xbox PDB, ADR-0002);
//! addresses are the PC executable's (ADR-0003). Everything here works in
//! Havok units (1 Havok unit = [`crate::HAVOK_UNIT`] game units) unless a
//! function takes a `unit` scale.
//!
//! This module: the world's constants and solver settings
//! (`hkpWorld::hkpWorld` `00c95a80`, `hkpWorldCinfo` defaults `00c90b80`,
//! Bethesda's `00c681c0`, `bhkWorld::ScaleSolverInfo` `00c66a00`).

use crate::Vec3;

/// `[HAVOK] fMaxTime` (setting `01267b38`; 0.016 in `Fallout_default.ini`,
/// the executable's default is 1/60): the length of one Havok step at
/// normal speed, and the world's `expectedMinPsiDeltaTime`.
pub const MAX_TIME: f32 = 0.016;
/// The world's gravity: set by the cells' `InitHavok` (`00554010`
/// exterior, `00552dc0` interior) from `011ca158` (written by `00f4b550`).
pub const GRAVITY: Vec3 = [0.0, 0.0, -98.1];
/// `hkpWorldCinfo` solver settings kept from Havok's defaults (`00c90b80`):
/// tau, damping, iterations (solver substeps before `00c66a00` rescales
/// them) and micro steps.
pub const SOLVER_TAU: f32 = 0.6;
pub const SOLVER_DAMPING: f32 = 1.0;
pub const SOLVER_ITERATIONS: i32 = 4;
pub const SOLVER_MICRO_STEPS: i32 = 1;
/// `fHavokTauRatio` (game setting at `011d1320`, copied to `011afe60` by
/// `0086f260`; default 0.5).
pub const TAU_RATIO: f32 = 0.5;
/// `hkpWorldCinfo::m_deactivationReferenceDistance` (`+0xa8`, Havok's
/// default `00c90b80`, 0x3ca3d70a).
pub const DEACTIVATION_REFERENCE_DISTANCE: f32 = 0.02;
/// `m_collisionTolerance` (`+0x54`, both cinfos).
pub const COLLISION_TOLERANCE: f32 = 0.1;
/// `m_contactRestingVelocity` (`+0x24`): Bethesda's FLT_MAX (`00c681c0`).
pub const CONTACT_RESTING_VELOCITY: f32 = f32::MAX;
/// `m_expectedMaxLinearVelocity` (`+0x60`, Havok's).
pub const EXPECTED_MAX_LINEAR_VELOCITY: f32 = 200.0;
/// `m_maxConstraintViolation` (`+0x88`, Havok's): FLT_MAX.
pub const MAX_CONSTRAINT_VIOLATION: f32 = f32::MAX;
/// `m_frameMarkerPsiSnap` (`+0xdc`).
pub const FRAME_MARKER_PSI_SNAP: f32 = 0.0001;
/// `m_enableDeactivation` (`+0xd0`) and `m_enableSimulationIslands`
/// (`+0xd2`): both on, so the world wants deactivation (`+0xd5`,
/// `00c95a80`).
pub const WANT_DEACTIVATION: bool = true;
/// `m_shouldActivateOnRigidBodyTransformChange` (`+0xa7`): Bethesda's 0.
pub const ACTIVATE_ON_TRANSFORM_CHANGE: bool = false;

/// hkUFloat8's values (the table at `010c7948`, 256 floats, read from the
/// executable): a motion's most linear and angular speed are kept as an
/// index into it (`hkpMotion` `+0xbc`, `+0xbd`).
pub const UFLOAT8: [f32; 256] = [
    0.0, 0.01, 0.02, 0.03, 0.04, 0.05, 0.06, 0.07, 0.075, 0.08, 0.085, 0.091, 0.098, 0.104, 0.111,
    0.119, 0.127, 0.136, 0.145, 0.155, 0.166, 0.177, 0.19, 0.203, 0.217, 0.231, 0.247, 0.264,
    0.282, 0.302, 0.323, 0.345, 0.368, 0.394, 0.421, 0.45, 0.481, 0.514, 0.549, 0.587, 0.627, 0.67,
    0.716, 0.765, 0.818, 0.874, 0.934, 0.998, 1.067, 1.14, 1.218, 1.302, 1.391, 1.487, 1.589,
    1.698, 1.815, 1.939, 2.072, 2.215, 2.367, 2.529, 2.703, 2.889, 3.087, 3.299, 3.526, 3.768,
    4.027, 4.304, 4.599, 4.915, 5.253, 5.613, 5.999, 6.411, 6.851, 7.322, 7.825, 8.362, 8.937,
    9.551, 10.21, 10.91, 11.66, 12.46, 13.31, 14.23, 15.2, 16.25, 17.37, 18.56, 19.83, 21.2, 22.65,
    24.21, 25.87, 27.65, 29.55, 31.57, 33.74, 36.06, 38.54, 41.18, 44.01, 47.04, 50.27, 53.72,
    57.41, 61.35, 65.57, 70.07, 74.88, 80.03, 85.52, 91.4, 97.68, 104.4, 111.6, 119.2, 127.4,
    136.2, 145.5, 155.5, 166.2, 177.6, 189.8, 202.8, 216.8, 231.7, 247.6, 264.6, 282.7, 302.2,
    322.9, 345.1, 368.8, 394.1, 421.2, 450.1, 481.1, 514.1, 549.4, 587.2, 627.5, 670.6, 716.6,
    765.9, 818.5, 874.7, 934.8, 999.0, 1068.0, 1141.0, 1219.0, 1303.0, 1393.0, 1488.0, 1590.0,
    1700.0, 1816.0, 1941.0, 2074.0, 2217.0, 2369.0, 2532.0, 2706.0, 2892.0, 3090.0, 3303.0, 3530.0,
    3772.0, 4031.0, 4308.0, 4604.0, 4920.0, 5258.0, 5619.0, 6005.0, 6418.0, 6858.0, 7329.0, 7833.0,
    8371.0, 8946.0, 9560.0, 10217.0, 10919.0, 11669.0, 12470.0, 13327.0, 14242.0, 15220.0, 16266.0,
    17383.0, 18577.0, 19853.0, 21217.0, 22674.0, 24231.0, 25896.0, 27674.0, 29575.0, 31607.0,
    33778.0, 36098.0, 38577.0, 41227.0, 44059.0, 47085.0, 50319.0, 53775.0, 57469.0, 61416.0,
    65635.0, 70143.0, 74961.0, 80110.0, 85612.0, 91493.0, 97777.0, 104493.0, 111670.0, 119340.0,
    127538.0, 136298.0, 145660.0, 155664.0, 166356.0, 177783.0, 189994.0, 203044.0, 216991.0,
    231895.0, 247823.0, 264846.0, 283037.0, 302478.0, 323254.0, 345457.0, 369186.0, 394544.0,
    421644.0, 450605.0, 481556.0, 514632.0, 549980.0, 587757.0, 628128.0, 671272.0, 717379.0,
    766654.0, 819313.0, 875589.0, 935730.0, 1000002.0,
];

/// A float as hkUFloat8 (`00ca9360`): 0 at or under 0.01's bits less one,
/// else a binary search of the table on the floats' bit patterns (as
/// integers), first halving at 216.8, then six halvings, then one up when
/// the value is still above and not at the last entry.
// Translated from 00ca9360 (decompiled, FalloutNV.exe 1.4.0.525)
pub fn ufloat8(value: f32) -> u8 {
    let v = value.to_bits() as i32;
    if v <= 0x3c23_d709 {
        return 0;
    }
    let bits = |i: usize| UFLOAT8[i].to_bits() as i32;
    let (mut lo, mut hi) = if v < 0x4358_ccce {
        (0, 128)
    } else {
        (128, 256)
    };
    for _ in 0..6 {
        let mid = (lo + hi) / 2;
        if bits(mid) < v {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    let mut i = (lo + hi) / 2;
    if bits(i) < v && i < 255 {
        i += 1;
    }
    i as u8
}

/// hkHalf: a float's upper 16 bits (stored by truncation, `00c95a80`; read
/// back as the bits shifted up, `00d28a30`).
pub fn half(value: f32) -> u16 {
    (value.to_bits() >> 16) as u16
}

pub fn from_half(h: u16) -> f32 {
    f32::from_bits(u32::from(h) << 16)
}

/// One deactivation class's thresholds (`hkpSolverInfo::DeactivationInfo`,
/// 0x1c bytes from solver info `+0x64`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DeactivationInfo {
    pub linear_velocity_threshold_inv: f32,
    pub angular_velocity_threshold_inv: f32,
    pub slow_object_velocity_multiplier: f32,
    pub relative_sleep_velocity_threshold: f32,
    /// Squared distance a body may stray from its reference position, for
    /// the frequent check (0) and the 16-step check (1).
    pub max_dist_sqrd: [f32; 2],
    /// Squared distance of its rotation from the reference one (hkHalf).
    pub max_rot_sqrd: [u16; 2],
}

/// The world's solver settings (`hkpSolverInfo`, world `+0x1e0`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolverInfo {
    pub tau: f32,
    pub damping: f32,
    pub friction_tau: f32,
    pub damp_div_tau: f32,
    pub tau_div_damp: f32,
    pub damp_div_friction_tau: f32,
    pub friction_tau_div_damp: f32,
    pub integrate_velocity_factor: f32,
    pub contact_resting_velocity: f32,
    pub deactivation: [DeactivationInfo; 6],
    /// Solver substeps (`m_numSteps`, `+0x114`) and their inverse.
    pub num_steps: i32,
    pub inv_num_steps: f32,
    pub num_micro_steps: i32,
    pub inv_num_micro_steps: f32,
    pub max_constraint_violation_sqrd: f32,
    /// `m_deactivationNumInactiveFramesSelectFlag[2]` (`+0x125`) and
    /// `m_deactivationIntegrateCounter` (`+0x127`); all 0 at the start
    /// (cinfo `+0xa4`..`+0xa6`, `00c90b80`).
    pub select_flags: [u8; 2],
    pub integrate_counter: u8,
}

impl Default for SolverInfo {
    fn default() -> Self {
        SolverInfo::new()
    }
}

impl SolverInfo {
    /// As the world's constructor leaves it (`00c95a80`), from the cinfo in
    /// force: tau and damping through `setTauAndDamping` (`00c90e60`), the
    /// iterations and micro steps, and the six deactivation classes from
    /// |gravity| (9.81 when it's 0) and the reference distance.
    // Translated from 00c95a80 and 00c90e60 (decompiled, FalloutNV.exe 1.4.0.525)
    pub fn new() -> Self {
        let mut s = SolverInfo {
            tau: 0.0,
            damping: 0.0,
            friction_tau: 0.0,
            damp_div_tau: 0.0,
            tau_div_damp: 0.0,
            damp_div_friction_tau: 0.0,
            friction_tau_div_damp: 0.0,
            integrate_velocity_factor: 0.0,
            contact_resting_velocity: CONTACT_RESTING_VELOCITY,
            deactivation: [DeactivationInfo {
                linear_velocity_threshold_inv: 0.0,
                angular_velocity_threshold_inv: 0.0,
                slow_object_velocity_multiplier: 0.0,
                relative_sleep_velocity_threshold: 0.0,
                max_dist_sqrd: [0.0; 2],
                max_rot_sqrd: [0; 2],
            }; 6],
            num_steps: SOLVER_ITERATIONS,
            inv_num_steps: 1.0 / SOLVER_ITERATIONS as f32,
            num_micro_steps: SOLVER_MICRO_STEPS,
            inv_num_micro_steps: 1.0 / SOLVER_MICRO_STEPS as f32,
            max_constraint_violation_sqrd: MAX_CONSTRAINT_VIOLATION * MAX_CONSTRAINT_VIOLATION,
            select_flags: [0; 2],
            integrate_counter: 0,
        };
        s.set_tau_and_damping(SOLVER_TAU, SOLVER_DAMPING);
        let mut g = crate::vec::length(GRAVITY);
        if g == 0.0 {
            g = 9.81;
        }
        let d = DEACTIVATION_REFERENCE_DISTANCE;
        for (class, info) in s.deactivation.iter_mut().enumerate() {
            let (v, a) = match class {
                0 | 1 => (1.192_092_9e-7, 0.0),
                2 => (0.01, 0.08),
                3 => (0.017, 0.2),
                4 => (0.02, 0.3),
                _ => (0.025, 0.4),
            };
            let v = v * g;
            let inv = 1.0 / SOLVER_ITERATIONS as f32;
            info.slow_object_velocity_multiplier = 1.0 - inv * 0.016 * ((a * g) / v);
            info.linear_velocity_threshold_inv = 1.0 / v;
            info.angular_velocity_threshold_inv = 1.0 / (v * g * 0.1);
            info.relative_sleep_velocity_threshold = if a <= 0.0 {
                2.126_762_5e37
            } else {
                (inv * 0.016) / a
            };
            info.max_dist_sqrd = [d * d, d * 4.0 * d * 4.0];
            info.max_rot_sqrd = [half((d + d) * (d + d)), half(d * 8.0 * d * 8.0)];
        }
        s
    }

    /// `hkpSolverInfo::setTauAndDamping` (`00c90e60`): friction tau is half
    /// of tau; the four ratios; the velocity factor tau ÷ damping.
    // Translated from 00c90e60 (decompiled, FalloutNV.exe 1.4.0.525)
    pub fn set_tau_and_damping(&mut self, tau: f32, damping: f32) {
        self.tau = tau;
        self.damping = damping;
        self.friction_tau = 0.5 * tau;
        self.damp_div_tau = damping / tau;
        self.tau_div_damp = tau / damping;
        self.damp_div_friction_tau = damping / self.friction_tau;
        self.friction_tau_div_damp = self.friction_tau / damping;
        self.integrate_velocity_factor = tau / damping;
    }

    /// `bhkWorld::ScaleSolverInfo` (`00c66a00`, `[HAVOK] iUpdateType` 0),
    /// once a frame before the steps, with `step` the step length: with
    /// r = step ÷ `fMaxTime`, the substeps are max(2, trunc(4r)); tau is
    /// 0.6 (1 − k) + 0.6 k r with k = `fHavokTauRatio`; then damping ÷ tau
    /// and tau ÷ damping. Friction tau and the integrate velocity factor
    /// are left as the constructor set them.
    // Translated from 00c66a00 (decompiled, FalloutNV.exe 1.4.0.525)
    pub fn scale(&mut self, step: f32, tau_ratio: f32) {
        let r = step / MAX_TIME;
        let n = ((4.0 * r) as i32).max(2);
        self.num_steps = n;
        self.inv_num_steps = 1.0 / n as f32;
        let tau = (1.0 - tau_ratio) * 0.6 + 0.6 * tau_ratio * r;
        self.tau = tau;
        self.damp_div_tau = self.damping / tau;
        self.tau_div_damp = tau / self.damping;
    }

    /// `hkpSolverInfo::incrementDeactivationFlags` (Xbox PDB; inlined in
    /// `hkpSimulation::integrateInternal` `00cf8da0`), once a step before
    /// the islands are integrated.
    // Translated from 00cf8da0 (decompiled, FalloutNV.exe 1.4.0.525)
    pub fn increment_deactivation_flags(&mut self) {
        self.integrate_counter = self.integrate_counter.wrapping_add(1);
        let c = self.integrate_counter;
        if c.wrapping_sub(4) & 7 == 0 {
            self.select_flags[0] ^= 1;
        }
        if c & 7 == 0 {
            self.select_flags[0] ^= 2;
        }
        if c & 15 == 0 {
            self.integrate_counter = 0;
            self.select_flags[1] = 1 - self.select_flags[1];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_speed_table_holds_the_bodies_values() {
        // Ends, and the bottle's stored speeds (Havok units, rad/s).
        assert_eq!(UFLOAT8[0], 0.0);
        assert_eq!(UFLOAT8[1], 0.01);
        assert_eq!(UFLOAT8[255], 1_000_002.0);
        // The literals are the executable's floats bit for bit.
        assert_eq!(UFLOAT8[1].to_bits(), 0x3c23_d70a);
        assert_eq!(UFLOAT8[128].to_bits(), 0x4358_cccd);
        assert_eq!(UFLOAT8[255].to_bits(), 0x4974_2420);
        assert_eq!(UFLOAT8[ufloat8(1068.0) as usize], 1068.0);
        assert_eq!(UFLOAT8[ufloat8(31.57) as usize], 31.57);
        assert_eq!(UFLOAT8[ufloat8(216.8) as usize], 216.8);
        // Between two entries: the one above. Under 0.01: 0. Huge: 255.
        assert_eq!(UFLOAT8[ufloat8(1000.0) as usize], 1068.0);
        assert_eq!(ufloat8(0.005), 0);
        assert_eq!(ufloat8(1e9), 255);
        // Every entry encodes to itself.
        for (i, &v) in UFLOAT8.iter().enumerate().skip(1) {
            assert_eq!(ufloat8(v) as usize, i, "{v}");
        }
    }

    #[test]
    fn halves_keep_the_upper_bits() {
        assert_eq!(from_half(half(1.0)), 1.0);
        let h = from_half(half(0.0016));
        assert!(h <= 0.0016 && h > 0.0015, "{h}");
    }

    #[test]
    fn the_world_starts_with_havoks_solver_settings() {
        let s = SolverInfo::new();
        assert_eq!((s.tau, s.damping, s.friction_tau), (0.6, 1.0, 0.3));
        assert_eq!((s.num_steps, s.num_micro_steps), (4, 1));
        assert!((s.tau_div_damp - 0.6).abs() < 1e-7);
        // Deactivation: 0.02 and 0.08 Havok units for every class.
        for d in &s.deactivation {
            assert!((d.max_dist_sqrd[0] - 0.0004).abs() < 1e-9);
            assert!((d.max_dist_sqrd[1] - 0.0064).abs() < 1e-8);
            assert_eq!(d.max_rot_sqrd, [half(0.0016), half(0.0256)]);
        }
        // The medium class: velocity 0.017 g, angle 0.2.
        let m = &s.deactivation[3];
        assert!((m.linear_velocity_threshold_inv - 1.0 / (0.017 * 98.1)).abs() < 1e-6);
        assert!((m.relative_sleep_velocity_threshold - 0.004 / 0.2).abs() < 1e-7);
    }

    #[test]
    fn substeps_and_tau_follow_the_time_multiplier() {
        let mut s = SolverInfo::new();
        s.scale(MAX_TIME, TAU_RATIO);
        assert_eq!(s.num_steps, 4);
        assert!((s.tau - 0.6).abs() < 1e-6);
        // Half speed: 2 substeps, tau 0.45; quarter speed: still 2.
        s.scale(MAX_TIME * 0.5, TAU_RATIO);
        assert_eq!(s.num_steps, 2);
        assert!((s.tau - 0.45).abs() < 1e-6, "{}", s.tau);
        s.scale(MAX_TIME * 0.25, TAU_RATIO);
        assert_eq!(s.num_steps, 2);
        // Friction tau isn't refreshed.
        assert_eq!(s.friction_tau, 0.3);
    }

    #[test]
    fn deactivation_flags_turn_over_every_sixteen_steps() {
        let mut s = SolverInfo::new();
        let mut seen = Vec::new();
        for _ in 0..16 {
            s.increment_deactivation_flags();
            seen.push((s.integrate_counter, s.select_flags));
        }
        assert_eq!(seen[3], (4, [1, 0]));
        assert_eq!(seen[7], (8, [3, 0]));
        assert_eq!(seen[11], (12, [2, 0]));
        assert_eq!(seen[15], (0, [0, 1]));
    }
}
