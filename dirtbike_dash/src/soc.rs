use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write, Seek, SeekFrom, BufReader, BufRead, Read};
use fs2::FileExt;

use ndarray::{Array, Array2, arr2};
use num::pow;
use polyfit_rs::polyfit_rs::polyfit;

use crate::soc;

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

pub fn data_collection(voltage: f64, curve: Vec<f64>, v_buf: &mut Vec<f64>, c_buf: &mut Vec<f64>, initial: &mut bool) -> f64 {
    let mut soc_value;

    // defines the file and calculates the capacity using the curve. All relevant data now exists
    let file = OpenOptions::new().write(true).open("soctable").expect("failed read");
    let capacity = curve[0] + curve[1]*voltage + curve[2]*(pow(voltage, 2)) + curve[3]*(pow(voltage, 3)) + curve[4]*(pow(voltage, 4));

    let mut writer = BufWriter::new(file);

    v_buf.push(voltage);
    c_buf.push(capacity);

    if v_buf.len() >= 25 {
        writer.get_ref().lock_exclusive().expect("failed to lock");
        writer.seek(SeekFrom::Start(0)).expect("failed to seek");

        // Write all 25 voltages on line 1
        for v in v_buf.iter() {
            write!(writer, "{} ", v).expect("failed to write");
        }
        writeln!(writer, "").expect("failed to write newline");

        // Write all 25 calculated values on line 2
        for c in c_buf.iter() {
            write!(writer, "{} ", c).expect("failed to write");
        }
        writeln!(writer, "").expect("failed to write newline");

        writer.flush().expect("failed to flush");
        writer.get_ref().unlock().expect("failed to unlock");

        // Clear buffers for next cycle
        v_buf.clear();
        c_buf.clear();
    }

    if *initial == true {
        soc_value = ocv_calc(voltage, curve);
        *initial = false;
    } else {
        soc_value = cc_calc();
    }

    return soc_value;
    
}

fn ocv_calc(voltage: f64, curve: Vec<f64>) ->  f64 {
    let mut file = File::open("soctable");

    let soc = 0.0;
    return soc;
}

fn cc_calc() ->  f64 {
    let mut file = File::open("soctable");

    let soc = 0.0;
    return soc;
}

pub fn read_data() -> Array2<f64> {
    let mut file = File::open("soctable").expect("failed to open file");

    let mut contents = String::new();
    file.read_to_string(&mut contents).expect("failed to retrieve file contents");

    let content_values: Vec<f64> = contents
        .split_whitespace()
        .map(|c| c.parse().expect("failed to parse"))
        .collect();

    let data_array = Array::from_shape_vec((2, 25), content_values).expect("failed to create array");

    return data_array;
}