// EMRAX 268 MV throttle-curve / torque model.
//
// Read-only: everything here is computed from values the dash already receives
// over CAN (throttle %, motor RPM, pack voltage). It only estimates what the
// motor is doing and what it could do; nothing is ever sent anywhere.
//
// Pipeline, run once per backend tick (`TorqueModel::step`):
//   throttle %  →  throttle curve  →  ramp-rate limit  →  min(thermal I²t limit,
//   voltage limit at this RPM)  →  phase current  →  torque = Kt × I
//
// All torques are motor-shaft torque in Nm; all currents are phase A_rms.
// Constants from the EMRAX 268 datasheet v1.6 (line-to-line RMS values).

// ── motor constants ─────────────────────────────────────────────────────────
pub struct MotorParams {
    pub ke: f64,            // induced voltage, Vrms (line-line) per RPM
    pub kt: f64,            // torque constant, Nm / A_rms
    pub i_peak: f64,        // peak phase current, A_rms
    pub i_cont: f64,        // continuous phase current, A_rms
    pub peak_time_s: f64,   // how long i_peak may be held from cold, s
    pub r_phase: f64,       // phase resistance @ 25 °C, Ω
    pub max_rpm: f64,       // rated max speed
    pub peak_rpm: f64,      // absolute max speed; no torque modelled past this
}

pub const EMRAX_268_MV: MotorParams = MotorParams {
    ke:          0.07823,
    kt:          0.43,
    i_peak:      500.0,
    i_cont:      220.0,
    peak_time_s: 60.0,
    r_phase:     0.00985,
    max_rpm:     4500.0,
    peak_rpm:    5500.0,
};

// ── model tuning ────────────────────────────────────────────────────────────
// throttle curve exponent: 1.0 linear, > 1 progressive (softer start),
// < 1 digressive (aggressive low end)
pub const THROTTLE_EXPONENT: f64 = 1.5;
// fastest the modelled current may rise or fall, A/s (0 → peak in 0.25 s)
pub const RAMP_RATE_A_PER_S: f64 = 2000.0;
// fraction of the I²t budget used before the current limit starts derating
pub const DERATE_START: f64 = 0.8;
// SVPWM modulation headroom when turning DC bus volts into available Vrms
const MODULATION: f64 = 0.95;

// ── throttle curve ──────────────────────────────────────────────────────────
pub struct ThrottleCurve {
    pub exponent: f64,
}

impl ThrottleCurve {
    // throttle fraction (0..1) → fraction of the current limit (0..1)
    pub fn map(&self, throttle: f64) -> f64 {
        throttle.clamp(0.0, 1.0).powf(self.exponent)
    }
}

// ── voltage limit ───────────────────────────────────────────────────────────
// line-line Vrms the controller can produce from the DC bus
pub fn v_available(v_bus: f64) -> f64 {
    v_bus / std::f64::consts::SQRT_2 * MODULATION
}

// RPM where back-EMF uses up all available voltage (no field weakening modelled)
pub fn base_speed_rpm(p: &MotorParams, v_bus: f64) -> f64 {
    v_available(v_bus) / p.ke
}

// most current the bus can push against back-EMF at this RPM: (V − Ke·ω) / R
pub fn voltage_limited_current(p: &MotorParams, rpm: f64, v_bus: f64) -> f64 {
    if rpm.abs() >= p.peak_rpm {
        return 0.0;
    }
    ((v_available(v_bus) - p.ke * rpm.abs()) / p.r_phase).max(0.0)
}

// ── thermal I²t model ───────────────────────────────────────────────────────
// Heat above the continuous rating accumulates as ∫(I² − I_cont²)dt and drains
// at the same rate when running below it. The budget is what holding i_peak for
// peak_time_s from cold would use. Past DERATE_START of the budget the limit
// slides linearly from i_peak down to i_cont.
pub struct ThermalI2t {
    used: f64, // A²·s
}

impl ThermalI2t {
    pub fn new() -> Self {
        Self { used: 0.0 }
    }

    pub fn budget(p: &MotorParams) -> f64 {
        (p.i_peak.powi(2) - p.i_cont.powi(2)) * p.peak_time_s
    }

    pub fn used_fraction(&self, p: &MotorParams) -> f64 {
        self.used / Self::budget(p)
    }

    pub fn limit(&self, p: &MotorParams) -> f64 {
        let f = self.used_fraction(p);
        if f <= DERATE_START {
            p.i_peak
        } else {
            let t = ((f - DERATE_START) / (1.0 - DERATE_START)).min(1.0);
            p.i_peak - (p.i_peak - p.i_cont) * t
        }
    }

    pub fn update(&mut self, p: &MotorParams, current: f64, dt: f64) {
        self.used = (self.used + (current.powi(2) - p.i_cont.powi(2)) * dt)
            .clamp(0.0, Self::budget(p));
    }
}

// ── ramp-rate limiter ───────────────────────────────────────────────────────
pub struct RampLimiter {
    pub value: f64,
    pub rate: f64, // units per second
}

impl RampLimiter {
    pub fn step(&mut self, target: f64, dt: f64) -> f64 {
        let max_delta = self.rate * dt;
        self.value += (target - self.value).clamp(-max_delta, max_delta);
        self.value
    }
}

// ── full model ──────────────────────────────────────────────────────────────
#[derive(Debug, Default, Clone, Copy)]
pub struct TorqueState {
    pub current_a: f64,        // modelled phase current, A_rms
    pub torque_nm: f64,        // modelled shaft torque
    pub current_limit_a: f64,  // min(thermal, voltage) limit right now
    pub torque_limit_nm: f64,  // max shaft torque available right now
    pub thermal_used_pct: f64, // share of the I²t budget used
    pub voltage_limited: bool, // back-EMF, not heat, is the binding limit
}

pub struct TorqueModel {
    pub params: &'static MotorParams,
    pub curve: ThrottleCurve,
    thermal: ThermalI2t,
    ramp: RampLimiter,
}

impl TorqueModel {
    pub fn new(params: &'static MotorParams) -> Self {
        Self {
            params,
            curve: ThrottleCurve { exponent: THROTTLE_EXPONENT },
            thermal: ThermalI2t::new(),
            ramp: RampLimiter { value: 0.0, rate: RAMP_RATE_A_PER_S },
        }
    }

    // advance the model by dt seconds
    pub fn step(&mut self, throttle_pct: f64, rpm: f64, v_bus: f64, dt: f64) -> TorqueState {
        let p = self.params;

        let requested = self.curve.map(throttle_pct / 100.0) * p.i_peak;
        let ramped = self.ramp.step(requested, dt);

        let thermal_limit = self.thermal.limit(p);
        let voltage_limit = voltage_limited_current(p, rpm, v_bus);
        let limit = thermal_limit.min(voltage_limit);

        let current = ramped.min(limit);
        self.ramp.value = current; // don't let the ramp wind up past a limit
        self.thermal.update(p, current, dt);

        TorqueState {
            current_a: current,
            torque_nm: p.kt * current,
            current_limit_a: limit,
            torque_limit_nm: p.kt * limit,
            thermal_used_pct: self.thermal.used_fraction(p) * 100.0,
            voltage_limited: voltage_limit < thermal_limit,
        }
    }
}

// ── envelopes ───────────────────────────────────────────────────────────────
// Steady-state curves (no ramp, fixed current limit), for analysis/plotting.
// Pass p.i_peak for the peak envelope or p.i_cont for the continuous one.

// (throttle %, shaft torque Nm) at a given RPM and bus voltage
pub fn torque_vs_throttle(
    p: &MotorParams, curve: &ThrottleCurve, rpm: f64, v_bus: f64, i_limit: f64, steps: usize,
) -> Vec<(f64, f64)> {
    let cap = i_limit.min(voltage_limited_current(p, rpm, v_bus));
    (0..=steps)
        .map(|k| {
            let pct = 100.0 * k as f64 / steps as f64;
            let i = (curve.map(pct / 100.0) * i_limit).min(cap);
            (pct, p.kt * i)
        })
        .collect()
}

// (RPM, max shaft torque Nm) from 0 to peak_rpm at full throttle
pub fn torque_vs_speed(p: &MotorParams, v_bus: f64, i_limit: f64, steps: usize) -> Vec<(f64, f64)> {
    (0..=steps)
        .map(|k| {
            let rpm = p.peak_rpm * k as f64 / steps as f64;
            (rpm, p.kt * i_limit.min(voltage_limited_current(p, rpm, v_bus)))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: &MotorParams = &EMRAX_268_MV;

    #[test]
    fn curve_shapes() {
        let lin = ThrottleCurve { exponent: 1.0 };
        let prog = ThrottleCurve { exponent: 2.0 };
        let dig = ThrottleCurve { exponent: 0.5 };
        assert!((lin.map(0.5) - 0.5).abs() < 1e-9);
        assert!((prog.map(0.5) - 0.25).abs() < 1e-9);
        assert!((dig.map(0.25) - 0.5).abs() < 1e-9);
        assert_eq!(lin.map(1.5), 1.0);
        assert_eq!(lin.map(-0.2), 0.0);
    }

    #[test]
    fn voltage_limit_hits_zero_at_base_speed() {
        let base = base_speed_rpm(P, 400.0);
        assert!(voltage_limited_current(P, base * 0.999, 400.0) > 0.0);
        assert_eq!(voltage_limited_current(P, base, 400.0), 0.0);
        assert_eq!(voltage_limited_current(P, P.peak_rpm, 600.0), 0.0);
    }

    #[test]
    fn peak_held_derates_to_continuous() {
        let mut m = TorqueModel::new(P);
        let dt = 0.01;
        let mut t = 0.0;
        let mut s = TorqueState::default();
        // full throttle at standstill, 540 V, for 2 minutes
        while t < 120.0 {
            s = m.step(100.0, 0.0, 540.0, dt);
            t += dt;
            if t > 1.0 && t < 40.0 {
                assert!((s.current_a - P.i_peak).abs() < 1e-6, "full peak early on");
            }
            assert!(s.current_a <= P.i_peak + 1e-6 && (t < 1.0 || s.current_a >= P.i_cont));
        }
        // the derate approaches i_cont exponentially (τ ≈ 20 s), so near it, not on it
        assert!(s.current_a < P.i_cont * 1.05, "settles near continuous: {}", s.current_a);
    }

    #[test]
    fn ramp_limits_rise() {
        let mut m = TorqueModel::new(P);
        let s = m.step(100.0, 0.0, 540.0, 0.05);
        assert!((s.current_a - RAMP_RATE_A_PER_S * 0.05).abs() < 1e-6);
    }

    // `cargo test print_envelopes -- --nocapture` prints the tables
    #[test]
    fn print_envelopes() {
        let curve = ThrottleCurve { exponent: THROTTLE_EXPONENT };
        for v in [540.0, 352.0] {
            println!("\nV_bus {v} V  (base speed {:.0} rpm)", base_speed_rpm(P, v));
            println!("torque vs throttle @ 0 rpm   [throttle %, peak Nm, cont Nm]");
            let pk = torque_vs_throttle(P, &curve, 0.0, v, P.i_peak, 10);
            let ct = torque_vs_throttle(P, &curve, 0.0, v, P.i_cont, 10);
            for ((pct, tp), (_, tc)) in pk.iter().zip(&ct) {
                println!("  {pct:5.0}  {tp:7.1}  {tc:7.1}");
            }
            println!("torque vs speed, full throttle [rpm, peak Nm, cont Nm]");
            let pk = torque_vs_speed(P, v, P.i_peak, 11);
            let ct = torque_vs_speed(P, v, P.i_cont, 11);
            for ((rpm, tp), (_, tc)) in pk.iter().zip(&ct) {
                println!("  {rpm:5.0}  {tp:7.1}  {tc:7.1}");
            }
        }
    }
}
