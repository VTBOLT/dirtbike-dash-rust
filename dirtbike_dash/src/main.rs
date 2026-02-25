mod can;
mod backend;
mod gps;

use std::{
    env,
    thread,
    time::Duration,
};

fn main() {
    let iface = env::args().nth(1).unwrap_or_else(|| "can0".to_string());

    let gps = gps::new_gps_state();

    let iface_clone = iface.clone();
    thread::spawn(move || {
        if let Err(e) = can::run(&iface_clone) {
            eprintln!("[CAN] Fatal: {e}");
        }
    });

    gps::spawn(std::sync::Arc::clone(&gps));

    let backend = backend::new(gps);

    loop {
        thread::sleep(Duration::from_secs(1));
        let b = backend.lock().unwrap().clone();
        println!("=== Backend snapshot ===");
        println!("  Motor temp    : {:.1} °C",    b.motor_temp);
        println!("  Aux voltage   : {:.2} V",      b.aux_voltage);
        println!("  Aux %         : {:.1} %",      b.aux_percentage);
        println!("  Pack SOC      : {:.1} %",      b.pack_soc);
        println!("  Pack voltage  : {:.1} V",      b.pack_voltage);
        println!("  Pack current  : {:.1} A",      b.pack_current);
        println!("  High cell T   : {:.1} °C",    b.high_cell_temp);
        println!("  Low  cell T   : {:.1} °C",    b.low_cell_temp);
        println!("  BMS temp      : {:.1} °C",    b.bms_temp);
        println!("  MC temp       : {:.1} °C",    b.mc_temp);
        println!("  Motor speed   : {:.0} RPM",    b.motor_speed);
        println!("  Motor on      : {}",            b.motor_on);
        println!("  Bike status   : {}",            b.bike_status);
        println!("  Speed (motor) : {:.1} mph",     b.bike_speed_motor);
        println!("  Speed (GPS)   : {:.1} mph",     b.bike_speed_gps);
        println!("  MC fault      : {}",            b.mc_fault);
        println!("  BMS fault     : {}",            b.bms_fault);
        println!("  BMS warning   : {}",            b.bms_warning);
        println!("  BMS error     : {}",            b.bms_error);
        println!("  BMS err codes : 0x{:06X}",     b.bms_error_codes);
        println!("  BMS messages  : {:?}",          b.bms_error_code_string);
        println!("  GPS           : ({}, {})",       b.lat, b.lon);
        println!("  Altitude      : {:.1} m",       b.altitude_m);
        println!("  Heading       : {:?} °",        b.heading_deg);
        println!("  GPS timestamp : {:.3} s",       b.gps_timestamp_s);
        println!("  GPS fix valid : {}",             b.gps_fix_valid);
        println!("  GPS fix mode  : {}",             b.gps_fix_mode);
    }
}