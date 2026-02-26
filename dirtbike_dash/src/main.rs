mod can;
mod backend;
mod gps;

#[cfg(feature = "sim")]
mod sim;

use std::{
    env,
    thread,
    time::Duration,
};

fn main() {
    // Default to vcan0 when simulating, can0 otherwise
    let iface = env::args().nth(1).unwrap_or_else(|| {
        if cfg!(feature = "sim") { "vcan0".to_string() }
        else                     { "can0".to_string() }
    });

    // ── GPS ─────────────────────────────────────────────────────────────
    let gps = gps::new_gps_state();
    gps::spawn(std::sync::Arc::clone(&gps));

    // ── CAN reader — always runs when "can" feature is active ───────────
    //    With --features sim, "can" is implied, so can::run reads vcan0.
    {
        let iface_clone = iface.clone();
        thread::spawn(move || {
            if let Err(e) = can::run(&iface_clone) {
                eprintln!("[CAN] Fatal: {e}");
            }
        });
    }

    // ── CAN writer (simulator) — sends encoded frames to vcan0 ─────────
    #[cfg(feature = "sim")]
    {
        println!("[MAIN] Simulator mode — writing fake CAN frames to {iface}");
        sim::spawn();
    }

    // ── Backend (scales raw CAN data, merges GPS) ───────────────────────
    let backend = backend::new(gps);

    // ── Print loop ──────────────────────────────────────────────────────
    loop {
        thread::sleep(Duration::from_secs(1));
        let b = backend.lock().unwrap().clone();

        let status_label = match b.bike_status {
            0 => "OFF",
            1 => "Idle",
            2 => "Precharge",
            3 => "Ready",
            4 => "Active",
            5 => "FAULT",
            _ => "???",
        };

        print!("\x1B[2J\x1B[H"); // clear terminal
        println!("╔══════════════════════════════════════════╗");
        println!("║         DIRTBIKE DASH  —  {:>10}     ║", status_label);
        println!("╠══════════════════════════════════════════╣");
        println!("║  Motor temp    :  {:>7.1} °C             ║", b.motor_temp);
        println!("║  MC temp       :  {:>7.1} °C             ║", b.mc_temp);
        println!("║  BMS temp      :  {:>7.1} °C             ║", b.bms_temp);
        println!("║  High cell T   :  {:>7.1} °C             ║", b.high_cell_temp);
        println!("║  Low  cell T   :  {:>7.1} °C             ║", b.low_cell_temp);
        println!("╠══════════════════════════════════════════╣");
        println!("║  Pack SOC      :  {:>7.1} %              ║", b.pack_soc);
        println!("║  Pack voltage  :  {:>7.1} V              ║", b.pack_voltage);
        println!("║  Pack current  :  {:>7.1} A              ║", b.pack_current);
        println!("║  Aux voltage   :  {:>7.2} V              ║", b.aux_voltage);
        println!("║  Aux %         :  {:>7.1} %              ║", b.aux_percentage);
        println!("╠══════════════════════════════════════════╣");
        println!("║  Motor speed   :  {:>7.0} RPM            ║", b.motor_speed);
        println!("║  Speed (motor) :  {:>7.1} mph            ║", b.bike_speed_motor);
        println!("║  Speed (GPS)   :  {:>7.1} mph            ║", b.bike_speed_gps);
        println!("║  Motor on      :  {:>7}                ║", b.motor_on);
        println!("╠══════════════════════════════════════════╣");
        println!("║  MC fault      :  {:>7}                ║", b.mc_fault);
        println!("║  BMS fault     :  {:>7}                ║", b.bms_fault);
        println!("║  BMS warning   :  {:>7}                ║", b.bms_warning);
        println!("║  BMS error     :  {:>7}                ║", b.bms_error);
        println!("║  BMS err codes : 0x{:06X}                ║", b.bms_error_codes);
        if !b.bms_error_code_string.is_empty() {
            for msg in &b.bms_error_code_string {
                println!("║    ⚠ {:<36}║", msg);
            }
        }
        println!("╠══════════════════════════════════════════╣");
        println!("║  GPS           :  ({:.6}, {:.6})   ║", b.lat, b.lon);
        println!("║  Altitude      :  {:>7.1} m              ║", b.altitude_m);
        println!("║  Heading       :  {:>10}°            ║",
                 b.heading_deg.map(|h| format!("{:.1}", h)).unwrap_or_else(|| "---".into()));
        println!("║  GPS fix       :  {:>5} (mode {})         ║", b.gps_fix_valid, b.gps_fix_mode);
        println!("║  GPS time      :  {:>5} s                ║", b.gps_timestamp_s);
        println!("╚══════════════════════════════════════════╝");
    }
}