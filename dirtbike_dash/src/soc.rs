use fs2::FileExt;
use ndarray::{Array, Array2};
use num::pow;
use polyfit_rs::polyfit_rs::polyfit;
use round::{round_up};

use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write, Read},
    time::{Instant}
};

// creates a polynomial regression model to match the data points. Instead of using a standardized one, this should recalculate itself and generate a new one without need for update, at the cost of starup performance and some storage
pub fn ocv_curve(soc_data: Array2<f64>) -> Vec<f64> {
    let voltage_data: Vec<f64> = soc_data.row(0).to_vec();
    let cap_data: Vec<f64> = soc_data.row(1).to_vec();

    let ocv_coeffs = polyfit(&voltage_data, &cap_data, 4).expect("polyfit failed");

    return ocv_coeffs;
}


// might just nor be needed but im leaving it for the moment
// pub fn initiate_buffers(v_buf: &mut Vec<f64>, c_buf: &mut Vec<f64>) {
//     let soc_data = read_data();
//     let voltage_data: Vec<f64> = soc_data.row(0).to_vec();
//     let cap_data: Vec<f64> = soc_data.row(1).to_vec();

//     let v_buf = voltage_data;
//     let c_buf = cap_data;
// }

// responsible for most everything
pub fn data_collection(voltage: f64, curve: Vec<f64>, v_buf: &mut Vec<f64>, c_buf: &mut Vec<f64>, max_cap: &f64, current: &f64, initial_time: &Instant) -> f64 {
    let mut soc_value= 0.0;

    // pulls the curve generated from previous instances. The bike will never be on long enough to justify regenerating a new curve while online and polyfit is kinda bulky
    let capacity = curve[0] + curve[1]*voltage + curve[2]*(pow(voltage, 2)) + curve[3]*(pow(voltage, 3)) + curve[4]*(pow(voltage, 4));

    v_buf[round_up(capacity, 0) as usize] = voltage; // updates the buffer. Faster than file writes
    c_buf[round_up(capacity, 0) as usize] = capacity; // updates the buffer. Faster than file writes

    // calculates the ocv. It kinda bad but any other way to calculate the ocv once ten use coulomb counting would be much more performance taxing and I have plenty of memory to spare on the pi
    if initial_time.elapsed().as_secs_f64() <= 1.0 {
        soc_value = capacity;
    } else {
        soc_value = cc_calc(current, max_cap, &initial_time, &soc_value);
    }

    return soc_value;
    
}

// TODO
fn cc_calc(current: &f64, max_cap: &f64, initial_time: &Instant, initial_soc: &f64) ->  f64 {
    let mut soc = *initial_soc;
    let t = initial_time.elapsed().as_secs_f64();
    let last_time = 0.0;
    let last_current = 0.0;

    // using trapezoidal method. SOC was already approximate enough that I'm happy to not use standard integration acorss 1000 or so points
    let dt = t - last_time;
    let avg_current = (current + last_current) / 2.0;
    soc += (1.0/max_cap) * avg_current * dt;
    
    return soc;
}

// reads all the data on the file on startup. used for all initial calculations
pub fn read_soctable() -> Array2<f64> {
    let mut file = File::open("soctable").expect("failed to open file");

    // pulls the entire thing to a string
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("failed to retrieve file contents");

    // splits the string into a single vector
    let content_values: Vec<f64> = contents
        .split_whitespace()
        .map(|c| c.parse().expect("failed to parse"))
        .collect();

    // builds a 2x100 array from the data in the string
    let data_array = Array::from_shape_vec((3, 100), content_values).expect("failed to create array");

    return data_array;
}

// reads all the data on the file on startup. used for all initial calculations
pub fn read_battery_props() -> Vec<f64> {
    let mut file = File::open("socmax").expect("failed to open file");

    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("failed to retrieve file contents");

    // splits the string into a single vector
    let content_values: Vec<f64> = contents
        .split_whitespace()
        .map(|c| c.parse().expect("failed to parse"))
        .collect();

    return content_values;
}

// fn edit_max_cap(capacity: &f64) {
//     let file = OpenOptions::new().write(true).create(true).truncate(true).open("socmax").expect("failed read");

//     let mut writer = BufWriter::new(file);
//     writer.get_ref().lock_exclusive().expect("failed to lock");
//     writeln!(writer, "{}", capacity).expect("failed to write");
//     writer.get_ref().unlock().expect("failed to unlock");
// }

pub fn write_soc_table(voltages: &Vec<f64>, capacities: &Vec<f64>) {
    let file = OpenOptions::new().write(true).create(true).truncate(true).open("soctable").expect("failed read");

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