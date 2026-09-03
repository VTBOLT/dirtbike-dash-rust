use fs2::FileExt;
use ndarray::{Array2};
use num::pow;
use polyfit_rs::polyfit_rs::polyfit;
use round::{round_up};

use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write, Read},
    time::{Instant}
};

const POLY_DEGREE: usize = 3;

// creates a polynomial regression model to match the data points. Instead of using a standardized one, this should recalculate itself and generate a new one without need for update, at the cost of starup performance and some storage
// DEFAULT CURVE FALLBACK:
//y=4.1303−((1.8625)(10^(-4))(x))+((6.5547)(10^(−8))(x^2))−((1.9191)(10^(−11))(x^3))
pub fn ocv_curve(soc_data: Array2<f64>) -> Vec<f64> {
    let voltage_data: Vec<f64> = soc_data.row(0).to_vec();
    let cap_data: Vec<f64> = soc_data.row(1).to_vec();

    let ocv_coeffs = polyfit(&voltage_data, &cap_data, POLY_DEGREE).expect("polyfit failed");

    // round each coefficient to 3 decimals to keep the curve readable / stable
    let ocv_round: Vec<f64> = ocv_coeffs.iter().map(|&n| round_up(n, 3)).collect();
    for n in &ocv_round {
        print!("{} ", n);
    }

    return ocv_round;
}

// responsible for most everything
pub fn data_collection(voltage: f64, curve: Vec<f64>, v_buf: &mut Vec<f64>, c_buf: &mut Vec<f64>, max_cap: &f64, current: &f64, initial_time: &Instant, orientation: f64) -> f64 {
    let mut soc_value= 0.0;
    let voltage_cell = voltage/orientation;
    let mut capacity_inv = 0.0;

    // calculates the ocv. It kinda bad but any other way to calculate the ocv once ten use coulomb counting would be much more performance taxing and I have plenty of memory to spare on the pi
    if soc_value > 10.0 || soc_value <=0.0 {
        capacity_inv = curve[0] + curve[1]*voltage_cell + curve[2]*(pow(voltage_cell, 2)) + curve[3]*(pow(voltage_cell, 3));// + curve[4]*(pow(voltage_cell, 4)) + curve[5]*(pow(voltage_cell, 5)) + curve[6]*(pow(voltage_cell, 6)) + curve[7]*(pow(voltage_cell, 7)) + curve[8]*(pow(voltage_cell, 8)) + curve[9]*(pow(voltage_cell, 9));
        soc_value = 1.0-capacity_inv;
    } else {
        soc_value = cc_calc(current, max_cap, &initial_time, &soc_value);
    }

    if soc_value%10.0 == 1.0 {
    // pulls the curve generated from previous instances. The bike will never be on long enough to justify regenerating a new curve while online and polyfit is kinda bulky
    v_buf[round_up(voltage_cell, 0) as usize] = voltage; // updates the buffer. Faster than file writes
    c_buf[round_up(1.0-capacity_inv, 0) as usize] = 1.0-capacity_inv; // updates the buffer. Faster than file writes
    }


    return soc_value;
    
}

fn cc_calc(current: &f64, max_cap: &f64, initial_time: &Instant, initial_soc: &f64) ->  f64 {
    let mut soc = *initial_soc;
    let t = initial_time.elapsed().as_secs_f64();
    let last_time = 0.0;
    let last_current = 0.0;

    // using trapezoidal method. SOC was already approximate enough that I'm happy to not use standard integration acorss so many points
    let dt = t - last_time;
    let avg_current = (current + last_current) / 2.0;
    soc -= (1.0/max_cap) * avg_current * (dt/360.0);
    
    return soc;
}

// reads all the data on the file on startup. used for all initial calculations
pub fn read_soctable() -> Array2<f64> {
    let mut file = File::open("/home/aki/.local/share/dashboard/soctable.txt").expect("failed to open file");

    // pulls the entire thing to a string
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("failed to retrieve file contents");

    // splits the string into a single vector
    let content_values: Vec<f64> = contents
        .split_whitespace()
        .map(|c| c.parse().expect("failed to parse"))
        .collect();

    // builds a 2xN array from the data in the string (row 0 = voltages, row 1 = capacities)
    let cols = content_values.len() / 2;
    assert!(cols > 0 && content_values.len() == 2 * cols, "soctable.txt must contain two equal-length rows of numbers");
    let data_array = Array2::from_shape_vec((2, cols), content_values).expect("failed to create array");

    return data_array;
}

// reads all the data on the file on startup. used for all initial calculations
pub fn read_battery_props() -> Vec<f64> {
    let mut file = File::open("/home/aki/.local/share/dashboard/battery_props.txt").expect("failed to open file");

    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("failed to retrieve file contents");

    // splits the string into a single vector
    let content_values: Vec<f64> = contents
        .split_whitespace()
        .map(|c| c.parse().expect("failed to parse"))
        .collect();

    return content_values;
}

pub fn write_soc_table(voltages: &Vec<f64>, capacities: &Vec<f64>) {
    let file = OpenOptions::new().write(true).create(true).truncate(true).open("/home/aki/.local/dashboard/soctable.txt").expect("failed read");

    let mut writer = BufWriter::new(file);
    writer.get_ref().lock_exclusive().expect("failed to lock");
    for v in voltages.iter() {
        write!(writer, "{} ", v).expect("failed to write");
    }
    writeln!(writer, "").expect("failed to write newline");

    for c in capacities.iter() {
        write!(writer, "{} ", c).expect("failed to write");
    }
    writeln!(writer, "").expect("failed to write newline");
    writer.flush().expect("failed to flush");
    writer.get_ref().unlock().expect("failed to unlock");

}