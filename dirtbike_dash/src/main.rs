mod can;
mod backend;
mod gps;
mod soc;

#[cfg(feature = "sim")]
mod sim;

use std::{
    env,
    thread,
    time::{Duration, Instant},
};

#[cfg(feature = "release")]
use std::sync::{Arc, Mutex};

#[cfg(feature = "release")]
slint::include_modules!();

// human-readable label for backend::Backend.bike_status
#[cfg(feature = "release")]
fn status_label(s: i32) -> &'static str {
    match s {
        0 => "OFF",
        1 => "Idle",
        2 => "Precharge",
        3 => "Ready",
        4 => "Active",
        5 => "FAULT",
        _ => "???",
    }
}

// Owns the Slint window and pumps the event loop. Every ~33 ms a UI-thread timer
// reads the shared backend snapshot and copies it into the window's `data` /
// `bms-errors` properties. The backend update thread (spawned in `backend::new`)
// keeps running independently; this just samples its latest output.
#[cfg(feature = "release")]
fn run_ui(backend: Arc<Mutex<backend::Backend>>, initial_time: Instant) {
    use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};

    let ui = MainWindow::new().expect("failed to create window");

    // UI -> Rust: the 'q' key / shutdown callback quits the event loop
    ui.on_shutdown(|| {
        let _ = slint::quit_event_loop();
    });

    let ui_weak = ui.as_weak();
    let timer = Timer::default();
    timer.start(
        TimerMode::Repeated,
        Duration::from_millis(33),
        move || {
            let Some(ui) = ui_weak.upgrade() else { return };
            let b = backend.lock().unwrap().clone();

            ui.set_data(DashData {
                speed_mph: b.bike_speed_motor as f32,
                speed_gps: b.bike_speed_gps,
                soc: b.pack_soc as f32,
                pack_voltage: b.pack_voltage as f32,
                pack_current: b.pack_current as f32,
                aux_voltage: b.aux_voltage as f32,
                aux_percent: b.aux_percentage as f32,
                motor_temp: b.motor_temp as f32,
                mc_temp: b.mc_temp as f32,
                bms_temp: b.bms_temp as f32,
                high_cell_temp: b.high_cell_temp as f32,
                low_cell_temp: b.low_cell_temp as f32,
                motor_rpm: b.motor_speed as f32,
                throttle: b.throttle as f32,
                status: status_label(b.bike_status).into(),
                motor_on: b.motor_on,
                mc_fault: b.mc_fault,
                bms_fault: b.bms_fault,
                bms_warning: b.bms_warning,
                bms_error: b.bms_error,
                bms_error_codes: format!("0x{:06X}", b.bms_error_codes).into(),
                gps_pos: format!("{:.5}, {:.5}", b.lat, b.lon).into(),
                altitude_m: b.altitude_m as f32,
                heading: b
                    .heading_deg
                    .map(|h| format!("{h:.1}"))
                    .unwrap_or_else(|| "---".into())
                    .into(),
                gps_fix: b.gps_fix_valid,
                gps_fix_mode: b.gps_fix_mode as i32,
                gps_time_s: b.gps_timestamp_s as f32,
                time_active: initial_time.elapsed().as_secs_f32(),
            });

            let errs: Vec<SharedString> = b
                .bms_error_code_string
                .iter()
                .map(SharedString::from)
                .collect();
            ui.set_bms_errors(ModelRc::new(VecModel::from(errs)));
        },
    );

    ui.run().expect("event loop failed");
    drop(timer);
}

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

    // hands the shared snapshot to the Slint window; blocks on the event loop
    #[cfg(feature = "release")]
    run_ui(std::sync::Arc::clone(&backend), initial_time);


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