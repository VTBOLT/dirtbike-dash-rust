mod can;
mod backend;
mod gps;
mod soc;
mod build;

#[cfg(feature = "sim")]
mod sim;

use std::{
    env,
    thread,
    time::{Duration, Instant},
};

#[cfg(feature = "release")]
slint::include_modules!();

fn main() {
    // starts a system time clock
    let initial_time = Instant::now();

    // if simulating, check vcan. DO NOT pass sim an argument for deployment, this will cause it to break
    let iface = env::args().nth(1).unwrap_or_else(|| {
        if cfg!(feature = "sim") { "vcan0".to_string() }
        else                     { "can0".to_string() }
    });

    // just grabs gps data\
    // gps also was a lot of non-human code, probably why it doesn't work
    let gps = gps::new_gps_state();
    gps::spawn(std::sync::Arc::clone(&gps));

    // can error reader. can is optional so others can test build on windows but it really isn't functional without it.
    {
        let iface_clone = iface.clone();
        thread::spawn(move || {
            if let Err(e) = can::run(&iface_clone) {
                eprintln!("[CAN] Fatal: {e}");
            }
        });
    }

    // makes the sim thread if passed with sim
    #[cfg(feature = "sim")]
    {
        println!("[MAIN] Simulator mode — writing fake CAN frames to {iface}");
        sim::spawn();
    }

    // assigns backend and adds the gps data. I may have done this wrong, this may also be why the gps data doesn't work but given the launch error I dont think so
    let backend = backend::new(gps, initial_time);


    #[cfg(feature = "release")] {
    let ui = MainWindow::new().unwrap();
    ui.run().unwrap();
    }


    // prints. please say it looks cool i put too much time into making it line up
    #[cfg(feature = "debug")]
    loop {
        thread::sleep(Duration::from_secs(1));
        let b: backend::Backend = backend.lock().unwrap().clone();

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
        println!("║  Time active   :  {:>7.1} secs           ║", initial_time.elapsed().as_secs_f64());
        println!("║  Motor temp    :  {:>7.1} °C             ║", b.motor_temp);
        println!("║  MC temp       :  {:>7.1} °C             ║", b.mc_temp);
        println!("║  BMS temp      :  {:>7.1} °C             ║", b.bms_temp);
        println!("║  High cell T   :  {:>7.1} °C             ║", b.high_cell_temp);
        println!("║  Low  cell T   :  {:>7.1} °C             ║", b.low_cell_temp);
        println!("╠══════════════════════════════════════════╣");
        println!("║  Pack SOC      :  {:>7.1} %              ║", b.pack_soc);
        println!("║  Pack voltage  :  {:>7.1} V              ║", b.pack_voltage);
        println!("║  Pack current  :  {:>7.1} A              ║", b.pack_current);
        println!("║  Aux voltage   :  {:>7.1} V              ║", b.aux_voltage);
        println!("║  Aux %         :  {:>7.1} %              ║", b.aux_percentage);
        println!("╠══════════════════════════════════════════╣");
        println!("║  Motor speed   :  {:>7.1} RPM            ║", b.motor_speed);
        println!("║  Speed (motor) :  {:>7.1} mph            ║", b.bike_speed_motor);
        println!("║  Speed (GPS)   :  {:>7.1} mph            ║", b.bike_speed_gps);
        println!("║  Motor on      :  {:>7}                ║", b.motor_on);
        println!("║  Throttle(%)   :  {:>7}                ║", b.throttle);
        println!("╠══════════════════════════════════════════╣");
        println!("║  MC fault      :  {:>7}                ║", b.mc_fault);
        println!("║  BMS fault     :  {:>7}                ║", b.bms_fault);
        println!("║  BMS warning   :  {:>7}                ║", b.bms_warning);
        println!("║  BMS error     :  {:>7}                ║", b.bms_error);
        println!("║  BMS err codes : 0x{:06X}                ║", b.bms_error_codes);
        if !b.bms_error_code_string.is_empty() { // bms errors aren't displayed by default, this picks them out and makes them a new line
            for msg in &b.bms_error_code_string {
                println!("║    ⚠ {:<36}║", msg);
            }
        }
        println!("╠══════════════════════════════════════════╣");
        println!("║  GPS           :  ({:2.3}, {:2.3})     ║", b.lat, b.lon);
        println!("║  Altitude      :  {:>7.1} m                   ║", b.altitude_m);
        println!("║  Heading       :  {:>7.1}°                 ║",
                 b.heading_deg.map(|h| format!("{:.1}", h)).unwrap_or_else(|| "---".into())); // random cluade line because I was obessed with making it pretty and couldn't figure it out. I don't know how it works and im afraid to touch it
        println!("║  GPS fix       :  {:>5} (mode {})             ║", b.gps_fix_valid, b.gps_fix_mode);
        println!("║  GPS time      :  {:>5} s                ║", b.gps_timestamp_s);
        println!("╚══════════════════════════════════════════╝");
    }
}