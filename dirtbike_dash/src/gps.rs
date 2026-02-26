
// currently about 80% claude because I have frankly no clue how the gps works but given that gps is bugging, I'll have to redo it all myself from the looks of things anyway

use std::{
    io::BufReader,
    net::TcpStream,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Debug, Default, Clone)]
pub struct GpsData {
    pub lat:         f64,
    pub lon:         f64,
    pub altitude_m:  f64,
    pub speed_mph:   f32,
    pub heading_deg: Option<f64>,
    pub timestamp_s: f64,
    pub fix_valid:   bool,
    pub fix_mode:    u8,
}

pub type SharedGpsState = Arc<Mutex<GpsData>>;

pub fn new_gps_state() -> SharedGpsState {
    Arc::new(Mutex::new(GpsData::default()))
}

/// Meters-per-second → miles-per-hour
const MPS_TO_MPH: f32 = 2.23694;

#[cfg(feature = "gps")]
pub fn gps_main(state: SharedGpsState) {
    use gpsd_proto::{get_data, handshake, Mode, ResponseData};

    loop {
        // Connect to gpsd (mirrors gpsmm gps_rec("localhost", DEFAULT_GPSD_PORT))
        let stream = match TcpStream::connect("localhost:2947") {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[GPS] Could not connect to gpsd: {e}. Retrying in 5s...");
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        let mut reader = BufReader::new(
            stream.try_clone().expect("Failed to clone TCP stream"),
        );
        let mut writer = stream;

        // Perform the gpsd handshake (VERSION → WATCH enable → DEVICES)
        // Mirrors gps_rec.stream(WATCH_ENABLE | WATCH_JSON) in the C++ code
        if let Err(e) = handshake(&mut reader, &mut writer) {
            eprintln!("[GPS] Handshake with gpsd failed: {e}. Retrying in 5s...");
            thread::sleep(Duration::from_secs(5));
            continue;
        }

        println!("[GPS] Connected to gpsd.");

        // Read loop (mirrors while(true) { gps_rec.waiting / gps_rec.read() })
        loop {
            match get_data(&mut reader) {
                Ok(ResponseData::Tpv(tpv)) => {
                    // Convert Mode enum → numeric fix mode
                    // (mirrors the MODE_2D || MODE_3D check in the C++ code)
                    let fix_mode: u8 = match tpv.mode {
                        Mode::Fix2d => 2,
                        Mode::Fix3d => 3,
                        Mode::NoFix => 0,
                    };

                    // Only update with a valid 2D or 3D fix
                    if fix_mode < 2 {
                        continue;
                    }

                    // Latitude and longitude must be present and finite
                    // (mirrors the !isnan check in the C++ code)
                    let lat = match tpv.lat {
                        Some(v) if v.is_finite() => v,
                        _ => continue,
                    };
                    let lon = match tpv.lon {
                        Some(v) if v.is_finite() => v,
                        _ => continue,
                    };

                    // Speed from gpsd is in m/s — convert to mph
                    let speed_mph: f32 = tpv
                        .speed
                        .filter(|v| v.is_finite())
                        .map(|v| v * MPS_TO_MPH)
                        .unwrap_or(0.0);

                    // Course over ground in degrees from true north
                    let heading_deg: Option<f64> = tpv
                        .track
                        .filter(|v| v.is_finite())
                        .map(|v| v as f64);

                    // Altitude in meters (fall back to 0 if unavailable)
                    let altitude_m: f64 = tpv
                        .alt
                        .filter(|v| v.is_finite())
                        .map(|v| v as f64)
                        .unwrap_or(0.0);

                    // gpsd "time" is ISO-8601 string; parse to epoch seconds
                    let timestamp_s: f64 = tpv
                        .time
                        .as_deref()
                        .and_then(parse_iso8601_epoch)
                        .unwrap_or(0.0);

                    *state.lock().unwrap() = GpsData {
                        lat,
                        lon,
                        altitude_m,
                        speed_mph,
                        heading_deg,
                        timestamp_s,
                        fix_valid: true,
                        fix_mode,
                    };
                }
                // Sky, Device, Pps, Gst — not needed
                Ok(_) => {}
                Err(e) => {
                    eprintln!("[GPS] Read error: {e}. Reconnecting...");
                    break; // drop connection, outer loop reconnects
                }
            }

            // Mirrors sleep(1) in the C++ version
            thread::sleep(Duration::from_secs(1));
        }
    }
}

// ── ISO-8601 timestamp helpers ──────────────────────────────────────────────

/// Parse a subset of ISO-8601 timestamps into seconds since Unix epoch.
/// gpsd sends strings like "2024-05-12T14:23:01.000Z".
/// For a full implementation, consider adding the `chrono` crate.
#[cfg(feature = "gps")]
fn parse_iso8601_epoch(s: &str) -> Option<f64> {
    let s = s.trim_end_matches('Z');
    let (date, time) = s.split_once('T')?;

    let mut dp = date.split('-');
    let year:  i64 = dp.next()?.parse().ok()?;
    let month: i64 = dp.next()?.parse().ok()?;
    let day:   i64 = dp.next()?.parse().ok()?;

    let (whole, frac) = if let Some((w, f)) = time.split_once('.') {
        let frac_secs: f64 = format!("0.{f}").parse().unwrap_or(0.0);
        (w, frac_secs)
    } else {
        (time, 0.0)
    };

    let mut tp = whole.split(':');
    let hour: i64 = tp.next()?.parse().ok()?;
    let min:  i64 = tp.next()?.parse().ok()?;
    let sec:  i64 = tp.next()?.parse().ok()?;

    let days = days_from_civil(year, month, day);
    Some(days as f64 * 86400.0 + hour as f64 * 3600.0 + min as f64 * 60.0 + sec as f64 + frac)
}

/// Civil date → days since Unix epoch (1970-01-01).
/// Adapted from Howard Hinnant's algorithm.
#[cfg(feature = "gps")]
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe as i64 * 365 + yoe as i64 / 4 - yoe as i64 / 100 + doy;
    era * 146097 + doe - 719468
}

// ── No-GPS fallback ─────────────────────────────────────────────────────────

#[cfg(not(feature = "gps"))]
pub fn gps_main(state: SharedGpsState) {
    println!("[GPS] gpsd support not compiled in — GPS state will remain default.");
    *state.lock().unwrap() = GpsData::default();
}

pub fn spawn(state: SharedGpsState) {
    let s = Arc::clone(&state);
    thread::spawn(move || gps_main(s));
}