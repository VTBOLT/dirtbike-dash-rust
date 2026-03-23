use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant}
};

use crate::can::{self, bms_errors, bms_warnings};
use crate::gps::SharedGpsState;
use crate::soc::{self};


// scaler constants
const MOTOR_TEMPERATURE_SCALE: f64     = 0.1;   // → C
const AUX_VOLTAGE_SCALE: f64           = 0.1;   // → V
const AUX_PERCENT_SCALE: f64           = 1.0;   // → %
const PACK_STATE_OF_CHARGE_SCALE: f64  = 100.0;   // → %
const PACK_VOLTAGE_SCALE: f64          = 0.1;   // → V
const PACK_CURRENT_SCALE: f64          = 0.1;   // → A
const HIGH_CELL_TEMP_SCALE: f64        = 0.1;   // → C
const LOW_CELL_TEMP_SCALE: f64         = 0.1;   // → C
const BMS_TEMPERATURE_SCALE: f64       = 0.1;   // → C
const MOTOR_SPEED_SCALE: f64           = 1.0;   // → RPM
const BIKE_SPEED_SCALE: f64            = 0.00953; // RPM → mph
const MC_TEMPERATURE_SCALE: f64        = 0.1;   // → C
const MAX_PACK_CURRENT: f64            = 1.0/175.0;

// struct to contain all the can data
#[derive(Debug, Default, Clone)]
pub struct Backend {
    pub motor_temp: f64,           // C
    pub aux_voltage: f64,          // V
    pub aux_percentage: f64,       // %
    pub pack_soc: f64,             // %
    pub high_cell_temp: f64,       // C
    pub low_cell_temp: f64,        // C
    pub bms_temp: f64,             // C
    pub mc_temp: f64,              // C
    pub pack_voltage: f64,         // V
    pub motor_on: bool,
    pub bike_status: i32,
    pub pack_current: f64,         // A
    pub motor_speed: f64,          // rpm
    pub bike_speed_motor: f64,     // mph
    pub bike_speed_gps: f32,       // mph
    pub mc_fault: bool,
    pub bms_error_codes: u32,
    pub bms_error: bool,
    pub bms_error_code_string: Vec<String>,
    pub bms_warning: bool,
    pub bms_fault: bool,
    pub lat: f64,
    pub lon: f64,
    pub altitude_m: f64,
    pub heading_deg: Option<f64>,
    pub gps_timestamp_s: f64,
    pub gps_fix_valid: bool,
    pub gps_fix_mode: u8,
    pub throttle: f64,
}

// check for errors as bool
pub fn get_error_code_strings(codes: u32) -> Vec<String> {
    let mut out = Vec::new();


    macro_rules! check {
        ($mask:expr, $label:expr) => {
            if codes & $mask != 0 {
                out.push($label.to_string());
            }
        };
    }

    // warnings
    check!(bms_warnings::DISCHARGE_LIMIT_ENFORCEMENT,  "Discharge Limit Enforcement");
    check!(bms_warnings::CELL_BALANCING_STUCK_OFF,     "Cell Balancing Stuck Off");
    check!(bms_warnings::WEAK_CELL,                    "Weak Cell");
    check!(bms_warnings::CURRENT_SENSOR,               "Current Sensor");
    check!(bms_warnings::WEAK_PACK,                    "Weak Pack");
    check!(bms_warnings::FAN_MONITOR,                  "Fan Monitor");
    check!(bms_warnings::CHARGER_SAFETY_RELAY,         "Charger Safety Relay");
    check!(bms_warnings::INTERNAL_HEATSINK_THERMISTOR, "Internal Heatsink Thermistor");
    check!(bms_warnings::OPEN_WIRING,                  "Open Wiring");
    check!(bms_warnings::THERMISTOR_FAULT,             "Thermistor Fault");
    check!(bms_warnings::EXTERNAL_COMMUNICATION,       "External Communication");
    check!(bms_warnings::CHARGE_LIMIT_ENFORCEMENT,     "Charge Limit Enforcement");

    // Errors
    check!(bms_errors::INTERNAL_HARDWARE,              "Internal Hardware");
    check!(bms_errors::INTERNAL_SOFTWARE,              "Internal Software");
    check!(bms_errors::HIGHEST_CELL_VOLTAGE_TOO_HIGH,  "Highest Cell Voltage Too High");
    check!(bms_errors::LOWEST_CELL_VOLTAGE_TOO_LOW,    "Lowest Cell Voltage Too Low");
    check!(bms_errors::PACK_TOO_HOT,                   "Pack Too Hot");
    check!(bms_errors::INTERNAL_COMMUNICATION,         "Internal Communication");
    check!(bms_errors::LOW_CELL_VOLTAGE,               "Low Cell Voltage");
    check!(bms_errors::HIGHEST_CELL_VOLTAGE_OVER_5V,   "Highest Cell Voltage Over 5v");
    check!(bms_errors::CELL_ASIC_FAULT,                "Cell ASIC Fault");
    check!(bms_errors::REDUNDANT_POWER_SUPPLY,         "Redundant Power Supply");
    check!(bms_errors::HIGH_VOLTAGE_ISOLATION,         "High Voltage Isolation");
    check!(bms_errors::INPUT_POWER_SUPPLY,             "Input Power Supply");

    out
}

// updates all the variables
pub fn update_vars(shared: Arc<Mutex<Backend>>, gps: SharedGpsState, initial_time: &Instant) {

    // read the data in the file to start
    let soc_data = soc::read_soctable();
    let battery_props = soc::read_battery_props();
    let max_cap = battery_props[0];

    // buffers for the data collection rows
    let mut v_buf: Vec<f64> = soc_data.row(0).to_vec();
    let mut c_buf: Vec<f64> = soc_data.row(1).to_vec();

    
    let ocv_curve = soc::ocv_curve(soc_data);
    loop {
        // raw can data
        let raw = can::DATA.lock().unwrap().clone();

        // raw gps data
        let g = gps.lock().unwrap().clone();

        // earlier access to certain values for the soc calculations. Otherwise there would be delay
        let voltage = raw.pack_voltage as f64 * PACK_VOLTAGE_SCALE;
        let current = raw.pack_current as f64 * PACK_CURRENT_SCALE;

        // soc calculations
        
        let 

        // updates soc values
        soc_value = soc::data_collection(voltage, ocv_curve.clone(), &mut v_buf, &mut c_buf, &max_cap, &current, &initial_time);

        // builds backend
        let next = Backend {

            // Temperatures
            motor_temp:     raw.motor_temperature as f64 * MOTOR_TEMPERATURE_SCALE,
            mc_temp:        raw.mc_temperature    as f64 * MC_TEMPERATURE_SCALE,
            bms_temp:       raw.bms_temperature   as f64 * BMS_TEMPERATURE_SCALE,
            high_cell_temp: raw.high_cell_temp    as f64 * HIGH_CELL_TEMP_SCALE,
            low_cell_temp:  raw.low_cell_temp     as f64 * LOW_CELL_TEMP_SCALE,

            // Aux battery
            aux_voltage:    raw.aux_voltage as f64 * AUX_VOLTAGE_SCALE,
            aux_percentage: raw.aux_percent         * AUX_PERCENT_SCALE,

            // Main pack
            pack_soc:       soc_value * PACK_STATE_OF_CHARGE_SCALE,
            pack_voltage:   voltage,
            pack_current:   current,

            // motor speed / bike speed from motor
            motor_speed:      raw.motor_speed as f64 * MOTOR_SPEED_SCALE,
            bike_speed_motor: raw.motor_speed as f64 * BIKE_SPEED_SCALE,

            // status
            motor_on:    raw.motor_on,
            bike_status: raw.bike_status,

            // faults
            bms_fault:             raw.bms_fault != 0,
            mc_fault:              raw.mc_fault,
            bms_error:             raw.bms_error,
            bms_warning:           raw.bms_warning,
            bms_error_codes:       raw.bms_error_codes,
            bms_error_code_string: get_error_code_strings(raw.bms_error_codes),

            // gps data
            lat:             g.lat,
            lon:             g.lon,
            altitude_m:      g.altitude_m,
            bike_speed_gps:  g.speed_mph,
            heading_deg:     g.heading_deg,
            gps_timestamp_s: g.timestamp_s,
            gps_fix_valid:   g.fix_valid,
            gps_fix_mode:    g.fix_mode,

            // dedicated rider readout vars
            throttle:        raw.pack_current as f64 * MAX_PACK_CURRENT,
        };

        *shared.lock().unwrap() = next;

        thread::sleep(Duration::from_millis(1));
    }
}

// builds a mutex for the gps data
pub fn new(gps: SharedGpsState, initial_time: Instant) -> Arc<Mutex<Backend>> {
    let shared: Arc<Mutex<Backend>> = Arc::new(Mutex::new(Backend::default()));
    let shared_clone = Arc::clone(&shared);

    thread::spawn(move || update_vars(shared_clone, gps, &initial_time));

    shared
}