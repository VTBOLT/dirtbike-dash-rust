//// VCAN Setup instructions
/// mandatory to use the sim lmao. probably only supports linux so..

// sudo modprobe vcan
// sudo ip link add dev vcan0 type vcan
// sudo ip link set up vcan0

use socketcan::{CanDataFrame, CanSocket, EmbeddedFrame, Socket, StandardId};
use std::{thread, time::Duration};

// can ids. I dont really want to iomport them even if it is more memory efficient
const AUX_BATTERY: u16          = 0x700;
const INFO: u16                 = 0x6B0;
const CELL_MAX_MIN_VOLTAGES: u16 = 0x6B3;
const MAIN_PACK_TEMP: u16       = 0x6B4;
const BMS_ERROR_CODES_ID: u16   = 0x6B6;
const MOTOR_TEMP: u16           = 0xA2;
const BMS_TEMP: u16             = 0x6B1;
const MC_TEMP: u16              = 0xA0;
const RPM: u16                  = 0xA5;
const MC_FAULTS: u16            = 0x0AB;
const INTERNAL_STATES: u16      = 0x0AA;
const ACC_SIGNAL: u16           = 0x706;

// BMS bit masks (same values as can.rs, used for fault-demo encoding)
const BMS_WARN_WEAK_CELL: u32   = 1 << 10;
const BMS_ERR_PACK_TOO_HOT: u32 = 1 << 7;

// Tick interval — 50 Hz matches a typical CAN bus update rate
const TICK_MS: u64 = 20;

// ── Helpers ─────────────────────────────────────────────────────────────────

// Build and send one CAN frame.  Panics on ID creation failure (all our
// IDs are valid standard IDs) but logs and continues on socket write errors.
fn send(sock: &CanSocket, id: u16, data: &[u8]) {
    let sid = StandardId::new(id).expect("invalid standard CAN ID");
    if let Some(frame) = CanDataFrame::new(sid, data) {
        if let Err(e) = sock.write_frame(&frame) {
            eprintln!("[SIM] write 0x{id:03X}: {e}");
        }
    }
}

/// Little-endian u16 → two bytes, matching `safe_u16(d, lo, hi)` decoding.
#[inline]
fn le(v: u16) -> [u8; 2] {
    v.to_le_bytes()
}

// Map the "human" bike_status (0–5) back to the raw VSM status code that
// `process_frame` expects on the INTERNAL_STATES frame.
//
// ```text
//   bike_status  raw status   motor_on (data[0] == 6)
//   0 (off)      — use ACC_SIGNAL data[0]=0 instead —
//   1 (idle)     0            false
//   2 (precharge)1            false
//   3 (ready)    4            false
//   4 (active)   6            true
//   5 (fault)    7            false
// ```
fn bike_status_to_raw_vsm(status: i32) -> u16 {
    match status {
        1 => 0,
        2 => 1,
        3 => 4,
        4 => 6,
        5 => 7,
        _ => 0,
    }
}

// ── Simulator core ──────────────────────────────────────────────────────────

/// Launch the simulator on its own thread.
pub fn spawn() {
    thread::spawn(run_sim);
}

fn run_sim() {
    // Open the virtual CAN socket
    let sock = match CanSocket::open("vcan0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[SIM] Failed to open vcan0: {e}\n\
                 [SIM] Run these commands first:\n\
                 [SIM]   sudo modprobe vcan\n\
                 [SIM]   sudo ip link add dev vcan0 type vcan\n\
                 [SIM]   sudo ip link set up vcan0"
            );
            return;
        }
    };

    println!("[SIM] CAN simulator started — sending frames on vcan0 at 50 Hz");

    // ── Mutable "physical" state ────────────────────────────────────────
    let mut tick: u64 = 0;

    // Temperatures (raw units — backend × 0.1 → °C)
    let mut motor_temp: u16  = 250;   // 25.0 °C
    let mut bms_temp: u16    = 280;   // 28.0 °C
    let mut mc_temp: u16     = 300;   // 30.0 °C
    let mut high_cell_t: u16 = 270;   // 27.0 °C
    let mut low_cell_t: u16  = 250;   // 25.0 °C

    // Electrical (raw units)
    let mut aux_voltage: u16  = 125;  // 12.5 V
    let mut pack_soc: u8      = 95;   // 95 %
    let mut pack_voltage: u16 = 1120; // 112.0 V
    let mut pack_current: i16 = 0;    // 0.0 A

    // Motor
    let mut motor_speed: i16 = 0;

    // Faults
    let mut bms_fault_byte: u8   = 0; // byte[5] of INFO frame, bit 4 = BMS fault
    let mut mc_fault: bool       = false;
    let mut bms_error_codes: u32 = 0;

    // Cell voltages (raw — × 0.0001 V)
    let mut highest_cell: u16 = 41500; // 4.15 V
    let mut lowest_cell: u16  = 40800; // 4.08 V

    // ── Phase timing (ticks @ 50 Hz) ────────────────────────────────────
    //    0 ..  100  (2 s)   Boot / off
    //  100 ..  250  (3 s)   Precharge
    //  250 ..  400  (3 s)   Ready
    //  400 .. 1400  (20 s)  Riding
    // 1400 .. 1650  (5 s)   Idle / cool-down
    // 1650 .. 1900  (5 s)   Fault demonstration
    // 1900 .. 2050  (3 s)   Fault clears → off
    const CYCLE_LEN: u64 = 2050;

    loop {
        let phase = tick % CYCLE_LEN;

        // Derive bike_status fresh each tick from the phase so it's always read
        let bike_status: i32;

        // ── Phase: Boot / Off ───────────────────────────────────────────
        if phase < 100 {
            bike_status = 0;
            motor_speed = 0;
            pack_current = 0;
            bms_fault_byte = 0;
            mc_fault = false;
            bms_error_codes = 0;
            if phase == 0 {
                motor_temp = 250;
                bms_temp = 280;
                mc_temp = 300;
                pack_soc = 95;
                pack_voltage = 1120;
                highest_cell = 41500;
                lowest_cell = 40800;
                aux_voltage = 125;
            }
        }
        // ── Phase: Precharge ────────────────────────────────────────────
        else if phase < 250 {
            bike_status = 2;
            pack_current = 5; // 0.5 A precharge trickle
        }
        // ── Phase: Ready ────────────────────────────────────────────────
        else if phase < 400 {
            pack_current = 0;
            bike_status = if phase == 399 { 4 } else { 3 };
        }
        // ── Phase: Riding ───────────────────────────────────────────────
        else if phase < 1400 {
            bike_status = 4;
            let ride_t = phase - 400; // 0 .. 1000

            if ride_t < 300 {
                // Accelerate 0 → 5000 RPM
                motor_speed = ((ride_t as f64 / 300.0) * 5000.0) as i16;
                pack_current = 800 + (ride_t as i16 / 2).min(400);
            } else if ride_t < 700 {
                // Cruise ~5000 RPM
                let wobble = ((ride_t as f64 * 0.05).sin() * 200.0) as i16;
                motor_speed = 5000 + wobble;
                pack_current = 350 + ((ride_t as f64 * 0.03).sin() * 50.0) as i16;
            } else {
                // Decelerate → 0
                let dt = ride_t - 700;
                let frac = 1.0 - (dt as f64 / 300.0);
                motor_speed = (frac * 5000.0) as i16;
                pack_current = if dt > 100 {
                    -(((dt as i16) - 100).min(200)) // regen
                } else {
                    (frac * 200.0) as i16
                };
            }

            // Temperatures rise
            if tick % 25 == 0 {
                motor_temp  = (motor_temp + 3).min(950);
                mc_temp     = (mc_temp + 2).min(750);
                bms_temp    = (bms_temp + 1).min(450);
                high_cell_t = (high_cell_t + 1).min(400);
                low_cell_t  = (low_cell_t + 1).min(380);
            }
            // SOC drains
            if tick % 100 == 0 && pack_soc > 5 {
                pack_soc -= 1;
                pack_voltage = pack_voltage.saturating_sub(3).max(800);
                highest_cell = highest_cell.saturating_sub(50);
                lowest_cell  = lowest_cell.saturating_sub(60);
            }
            // Aux drains
            if tick % 200 == 0 && aux_voltage > 110 {
                aux_voltage -= 1;
            }
        }
        // ── Phase: Idle / cool-down ─────────────────────────────────────
        else if phase < 1650 {
            bike_status = 1;
            motor_speed = 0;
            pack_current = 0;
            if tick % 50 == 0 {
                motor_temp = motor_temp.saturating_sub(5).max(250);
                mc_temp    = mc_temp.saturating_sub(3).max(300);
                bms_temp   = bms_temp.saturating_sub(2).max(280);
            }
        }
        // ── Phase: Fault demonstration ──────────────────────────────────
        else if phase < 1900 {
            let ft = phase - 1650;
            if ft < 125 {
                // Warning only
                bms_error_codes = BMS_WARN_WEAK_CELL;
                bike_status = 1;
                mc_fault = false;
            } else {
                // Warning + error
                bms_error_codes = BMS_WARN_WEAK_CELL | BMS_ERR_PACK_TOO_HOT;
                bike_status = 5;
                mc_fault = ft > 200;
            }
        }
        // ── Phase: Fault clears → off ───────────────────────────────────
        else {
            bms_error_codes = 0;
            mc_fault = false;
            bms_fault_byte = 0;
            bike_status = 0;
            motor_speed = 0;
            pack_current = 0;
        }

        // ════════════════════════════════════════════════════════════════
        //  Encode & send CAN frames — byte layout must match process_frame
        // ════════════════════════════════════════════════════════════════

        // ── 0x700 AUX_BATTERY ───────────────────────────────────────────
        {
            let v = le(aux_voltage);
            send(&sock, AUX_BATTERY, &[v[0], v[1]]);
        }

        // ── 0x6B0 INFO ──────────────────────────────────────────────────
        {
            let cur = (pack_current as u16).to_le_bytes();
            let vol = le(pack_voltage);
            send(&sock, INFO, &[cur[0], cur[1], vol[0], vol[1], pack_soc, bms_fault_byte, 0, 0]);
        }

        // ── 0x6B4 MAIN_PACK_TEMP ───────────────────────────────────────
        {
            let hi = le(high_cell_t);
            let lo = le(low_cell_t);
            send(&sock, MAIN_PACK_TEMP, &[hi[0], hi[1], lo[0], lo[1]]);
        }

        // ── 0x6B3 CELL_MAX_MIN_VOLTAGES ─────────────────────────────────
        {
            let hi = le(highest_cell);
            let lo = le(lowest_cell);
            send(&sock, CELL_MAX_MIN_VOLTAGES, &[0, 0, hi[0], hi[1], lo[0], lo[1]]);
        }

        // ── 0xA2 MOTOR_TEMP ─────────────────────────────────────────────
        {
            let t = le(motor_temp);
            send(&sock, MOTOR_TEMP, &[0, 0, 0, 0, t[0], t[1]]);
        }

        // ── 0x6B1 BMS_TEMP ──────────────────────────────────────────────
        {
            let t = le(bms_temp);
            send(&sock, BMS_TEMP, &[0, 0, 0, 0, t[0], t[1]]);
        }

        // ── 0xA5 RPM ────────────────────────────────────────────────────
        {
            let s = (motor_speed as u16).to_le_bytes();
            send(&sock, RPM, &[0, 0, s[0], s[1]]);
        }

        // ── 0xA0 MC_TEMP ────────────────────────────────────────────────
        {
            let t = le(mc_temp);
            send(&sock, MC_TEMP, &[t[0], t[1], t[0], t[1], t[0], t[1]]);
        }

        // ── 0x0AB MC_FAULTS ─────────────────────────────────────────────
        {
            let byte = if mc_fault { 0x01 } else { 0x00 };
            send(&sock, MC_FAULTS, &[byte, 0, 0, 0, 0, 0, 0, 0]);
        }

        // ── 0x0AA INTERNAL_STATES ───────────────────────────────────────
        if bike_status > 0 {
            let raw = bike_status_to_raw_vsm(bike_status);
            let r = raw.to_le_bytes();
            send(&sock, INTERNAL_STATES, &[r[0], r[1], 0, 0, 0, 0, 0, 0]);
        }

        // ── 0x706 ACC_SIGNAL ────────────────────────────────────────────
        {
            let byte = if bike_status == 0 { 0u8 } else { 1u8 };
            send(&sock, ACC_SIGNAL, &[byte]);
        }

        // ── 0x6B6 BMS_ERROR_CODES ───────────────────────────────────────
        //  Encoding: codes = data[2] | (data[0] << 8) | (data[1] << 16)
        {
            let d0 = ((bms_error_codes >> 8) & 0xFF) as u8;
            let d1 = ((bms_error_codes >> 16) & 0xFF) as u8;
            let d2 = (bms_error_codes & 0xFF) as u8;
            send(&sock, BMS_ERROR_CODES_ID, &[d0, d1, d2]);
        }

        tick += 1;
        thread::sleep(Duration::from_millis(TICK_MS));
    }
}

// as a reward for scrolling to the bottom, or because you searched the entire thing to find why it isnt working, this isn't human code. So I honestly couldn't tell you