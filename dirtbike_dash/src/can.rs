use std::sync::Mutex;

// constants
pub const FRAMES_TO_AVERAGE: usize = 10;
pub const CELL_VOLTAGE_SCALING: f64 = 0.0001;
pub const SAFEST_HIGH_CELL_VOLTAGE: f64 = 4.25;
pub const SAFEST_LOW_CELL_VOLTAGE: f64 = 3.0;

// set all the can ids
pub mod can_ids {
    pub const AUX_BATTERY: u32         = 0x700;
    pub const INFO: u32                = 0x6B0;
    pub const CELL_MAX_MIN_VOLTAGES: u32 = 0x6B3;
    pub const MAIN_PACK_TEMP: u32      = 0x6B4;
    pub const BMS_ERROR_CODES: u32     = 0x6B6;
    pub const MOTOR_TEMP: u32          = 0xA2;
    pub const BMS_TEMP: u32            = 0x6B1;
    pub const MC_TEMP: u32             = 0xA0;
    pub const RPM: u32                 = 0xA5;
    pub const SPEED: u32               = 0x00;
    pub const MC_FAULTS: u32           = 0x0AB;
    pub const INTERNAL_STATES: u32     = 0x0AA;
    pub const ACC_SIGNAL: u32          = 0x706;
}

// detecting bms errors & warnings
pub mod bms_warnings {
    pub const DISCHARGE_LIMIT_ENFORCEMENT: u32  = 1 << 0;
    pub const CHARGER_SAFETY_RELAY: u32          = 1 << 1;
    pub const INTERNAL_HEATSINK_THERMISTOR: u32  = 1 << 3;
    pub const CELL_BALANCING_STUCK_OFF: u32      = 1 << 9;
    pub const WEAK_CELL: u32                     = 1 << 10;
    pub const OPEN_WIRING: u32                   = 1 << 12;
    pub const CURRENT_SENSOR: u32                = 1 << 13;
    pub const WEAK_PACK: u32                     = 1 << 16;
    pub const FAN_MONITOR: u32                   = 1 << 17;
    pub const THERMISTOR_FAULT: u32              = 1 << 18;
    pub const EXTERNAL_COMMUNICATION: u32        = 1 << 19;
    pub const CHARGE_LIMIT_ENFORCEMENT: u32      = 1 << 23;
}

pub mod bms_errors {
    pub const INTERNAL_HARDWARE: u32              = 1 << 2;
    pub const INTERNAL_SOFTWARE: u32              = 1 << 4;
    pub const HIGHEST_CELL_VOLTAGE_TOO_HIGH: u32  = 1 << 5;
    pub const LOWEST_CELL_VOLTAGE_TOO_LOW: u32    = 1 << 6;
    pub const PACK_TOO_HOT: u32                   = 1 << 7;
    pub const INTERNAL_COMMUNICATION: u32         = 1 << 8;
    pub const LOW_CELL_VOLTAGE: u32               = 1 << 11;
    pub const HIGHEST_CELL_VOLTAGE_OVER_5V: u32   = 1 << 14;
    pub const CELL_ASIC_FAULT: u32                = 1 << 15;
    pub const REDUNDANT_POWER_SUPPLY: u32         = 1 << 20;
    pub const HIGH_VOLTAGE_ISOLATION: u32         = 1 << 21;
    pub const INPUT_POWER_SUPPLY: u32             = 1 << 22;
}

pub const ALL_BMS_ERRORS: u32 = bms_errors::INTERNAL_HARDWARE
    | bms_errors::INTERNAL_SOFTWARE
    | bms_errors::HIGHEST_CELL_VOLTAGE_TOO_HIGH
    | bms_errors::LOWEST_CELL_VOLTAGE_TOO_LOW
    | bms_errors::PACK_TOO_HOT
    | bms_errors::INTERNAL_COMMUNICATION
    | bms_errors::LOW_CELL_VOLTAGE
    | bms_errors::HIGHEST_CELL_VOLTAGE_OVER_5V
    | bms_errors::CELL_ASIC_FAULT
    | bms_errors::REDUNDANT_POWER_SUPPLY
    | bms_errors::HIGH_VOLTAGE_ISOLATION
    | bms_errors::INPUT_POWER_SUPPLY;

pub const ALL_BMS_WARNINGS: u32 = bms_warnings::DISCHARGE_LIMIT_ENFORCEMENT
    | bms_warnings::CHARGER_SAFETY_RELAY
    | bms_warnings::CELL_BALANCING_STUCK_OFF
    | bms_warnings::INTERNAL_HEATSINK_THERMISTOR
    | bms_warnings::WEAK_CELL
    | bms_warnings::CURRENT_SENSOR
    | bms_warnings::WEAK_PACK
    | bms_warnings::FAN_MONITOR
    | bms_warnings::THERMISTOR_FAULT
    | bms_warnings::EXTERNAL_COMMUNICATION
    | bms_warnings::OPEN_WIRING
    | bms_warnings::CHARGE_LIMIT_ENFORCEMENT;

// I havew this built to enable different modes maybe for the debug vs rider menu? either way, it passes a warning about it, don't worry abt it 
#[derive(Debug, Default, Clone)]
pub struct OurCanData {
    pub aux_voltage: u16,
    pub aux_percent: f64,
    pub pack_state_of_charge: u8,
    pub pack_voltage: u16,
    pub pack_current: i16,
    pub high_cell_temp: u16,
    pub low_cell_temp: u16,
    pub motor_temperature: i16,
    pub bms_temperature: u16,
    pub mc_temperature: u16,
    pub motor_speed: i16,
    pub bms_fault: u8,
    pub bms_error: bool,
    pub bms_warning: bool,
    pub bms_error_codes: u32,
    pub mc_fault: bool,
    pub motor_on: bool,
    pub bike_status: i32,
    pub highest_cell_voltage: u16,
    pub lowest_cell_voltage: u16,
}

pub static DATA: Mutex<OurCanData> = Mutex::new(OurCanData {
    aux_voltage: 0,
    aux_percent: 0.0,
    pack_state_of_charge: 0,
    pack_voltage: 0,
    pack_current: 0,
    high_cell_temp: 0,
    low_cell_temp: 0,
    motor_temperature: 0,
    bms_temperature: 0,
    mc_temperature: 0,
    motor_speed: 0,
    bms_fault: 0,
    bms_error: false,
    bms_warning: false,
    bms_error_codes: 0,
    mc_fault: false,
    motor_on: false,
    bike_status: 0,
    highest_cell_voltage: 0,
    lowest_cell_voltage: 0,
});

// gets average cell voltages
struct VoltageAverager {
    buf: [u16; FRAMES_TO_AVERAGE],
    idx: usize,
}

impl VoltageAverager {
    const fn new() -> Self {
        Self {
            buf: [0; FRAMES_TO_AVERAGE],
            idx: 0,
        }
    }
    fn push(&mut self, v: u16) {
        self.buf[self.idx] = v;
        self.idx = (self.idx + 1) % FRAMES_TO_AVERAGE;
    }
    fn average(&self) -> f64 {
        self.buf.iter().map(|&x| x as f64).sum::<f64>() / FRAMES_TO_AVERAGE as f64
    }
}


#[inline]
fn safe_u16(d: &[u8], lo: usize, hi: usize) -> u16 {
    let l = d.get(lo).copied().unwrap_or(0) as u16;
    let h = d.get(hi).copied().unwrap_or(0) as u16;
    l | (h << 8)
}

#[inline]
fn safe_byte(d: &[u8], i: usize) -> u8 {
    d.get(i).copied().unwrap_or(0)
}

// process actual can data
fn process_frame(
    id: u32,
    d: &[u8],
    data: &mut OurCanData,
    low_avg: &mut VoltageAverager,
    high_avg: &mut VoltageAverager,
) {
    match id { // I think this is right. hopefully.
        can_ids::AUX_BATTERY => {
            data.aux_voltage = safe_u16(d, 0, 1);
            data.aux_percent = data.aux_voltage as f64 / 2.5;
        }
        can_ids::INFO => {
            data.pack_state_of_charge = safe_byte(d, 4);
            data.bms_fault = safe_byte(d, 5) & 0b0010000;
            data.pack_current = safe_u16(d, 0, 1) as i16;
            data.pack_voltage = safe_u16(d, 2, 3);
        }
        can_ids::MAIN_PACK_TEMP => {
            data.high_cell_temp = safe_u16(d, 0, 1);
            data.low_cell_temp  = safe_u16(d, 2, 3);
        }
        can_ids::CELL_MAX_MIN_VOLTAGES => {
            data.highest_cell_voltage = safe_u16(d, 2, 3);
            data.lowest_cell_voltage  = safe_u16(d, 4, 5);
            high_avg.push(data.highest_cell_voltage);
            low_avg.push(data.lowest_cell_voltage);
        }
        can_ids::MOTOR_TEMP => {
            data.motor_temperature = safe_u16(d, 4, 5) as i16;
        }
        can_ids::BMS_TEMP => {
            data.bms_temperature = safe_u16(d, 4, 5);
        }
        can_ids::RPM | can_ids::SPEED => {
            data.motor_speed = safe_u16(d, 2, 3) as i16;
        }
        can_ids::MC_TEMP => {
            let r1 = safe_u16(d, 0, 1);
            let r2 = safe_u16(d, 2, 3);
            let r3 = safe_u16(d, 4, 5);
            data.mc_temperature = r1.max(r2).max(r3);
        }
        can_ids::MC_FAULTS => {
            data.mc_fault = d.iter().any(|&b| b != 0);
        }
        can_ids::INTERNAL_STATES => {
            let status = safe_u16(d, 0, 1);
            data.bike_status = match status {
                0       => 1,
                1 | 2 | 3 => 2,
                4 | 5   => 3,
                6       => 4,
                7       => 5,
                _       => data.bike_status,
            };
            data.motor_on = safe_byte(d, 0) == 6;
        }
        can_ids::BMS_ERROR_CODES => {
            // Layout: data[2] | data[0]<<8 | data[1]<<16  (from can.cpp)
            data.bms_error_codes =
                (safe_byte(d, 2) as u32)
                | ((safe_byte(d, 0) as u32) << 8)
                | ((safe_byte(d, 1) as u32) << 16);

            data.bms_error   = (data.bms_error_codes & ALL_BMS_ERRORS)   != 0;
            data.bms_warning = (data.bms_error_codes & ALL_BMS_WARNINGS) != 0;

            // Suppress transient voltage faults when cells are back in safe range
            if data.bms_error_codes & bms_errors::HIGHEST_CELL_VOLTAGE_TOO_HIGH != 0
                && (high_avg.average() * CELL_VOLTAGE_SCALING) <= SAFEST_HIGH_CELL_VOLTAGE
            {
                data.bms_error_codes &= !bms_errors::HIGHEST_CELL_VOLTAGE_TOO_HIGH;
            }
            if data.bms_error_codes & bms_errors::LOWEST_CELL_VOLTAGE_TOO_LOW != 0
                && (low_avg.average() * CELL_VOLTAGE_SCALING) >= SAFEST_LOW_CELL_VOLTAGE
            {
                data.bms_error_codes &= !bms_errors::LOWEST_CELL_VOLTAGE_TOO_LOW;
            }
        }
        can_ids::ACC_SIGNAL => {
            if safe_byte(d, 0) == 0 {
                data.bike_status = 0;
            } else if data.bike_status == 0 {
                data.bike_status = safe_byte(d, 0) as i32;
            }
        }

        unknown_id => {
            // hey we have a whole project for this now :D
            eprintln!("[CAN] Unknown frame ID: 0x{unknown_id:03X}  data: {d:?}");
        }
    }
}

// just reads can
#[cfg(feature = "can")]
pub fn run(iface: &str) -> anyhow::Result<()> {
    use socketcan::{CanFrame, CanSocket, EmbeddedFrame, Frame, Socket};

    // opens a can socket and assigns raw data
    let sock = CanSocket::open(iface)?;
    println!("[CAN] Opened interface: {iface}");

    // pulls a new number from each averager
    let mut low_avg  = VoltageAverager::new();
    let mut high_avg = VoltageAverager::new();

    loop {
        match sock.read_frame() { // reads a single frame
            Ok(CanFrame::Data(frame)) => {
                let id = frame.raw_id(); // picks out the id 
                let d  = frame.data(); // picks out the data matching the id

                // sets a data lock. if you don't get how this works, I would recommend the rust documentation. it contains a protected copy of the data basically
                let mut guard = DATA.lock().unwrap();
                process_frame(id, d, &mut guard, &mut low_avg, &mut high_avg);
            }
            Ok(_) => {} // can error handler
            Err(e) => {
                eprintln!("[CAN] Read error: {e}");
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
    }
}

/// does not contain can capabilities
#[cfg(not(feature = "can"))]
pub fn run(_iface: &str) -> anyhow::Result<()> {
    println!("[CAN] SocketCAN not available — run() is not available.");
    Ok(())
}