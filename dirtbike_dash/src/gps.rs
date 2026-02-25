use std::{
    sync::{Arc, Mutex},
    thread,
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

#[cfg(feature = "gps")]
pub fn gps_main(state: SharedGpsState) {
    use gpsd_client::{Client, Mode};
    use std::time::Duration;

    loop {
        // Connect to gpsd (mirrors gpsmm gps_rec("localhost", DEFAULT_GPSD_PORT))
        let mut client = match Client::connect("localhost:2947") {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[GPS] Could not connect to gpsd: {e}. Retrying in 5s...");
                thread::sleep(Duration::from_secs(5));
                continue;
            }
        };

        // Enable JSON streaming (mirrors gps_rec.stream(WATCH_ENABLE | WATCH_JSON))
        if let Err(e) = client.watch(true) {
            eprintln!("[GPS] Failed to start watch: {e}");
            thread::sleep(Duration::from_secs(5));
            continue;
        }

        println!("[GPS] Connected to gpsd.");

        // Read loop (mirrors while(true) { gps_rec.waiting / gps_rec.read() })
        loop {
            match client.next_message() {
                Ok(Some(msg)) => {
                    // Only update when we have a valid 2D or 3D fix with real coords
                    // (mirrors the MODE_2D || MODE_3D + !isnan check)
                    if let Some(fix) = msg.fix {
                        let fix_mode = match fix.mode {
                            Mode::Fix2D => 2,
                            Mode::Fix3D => 3,
                            _           => 0,
                        };
                        let has_fix = fix_mode >= 2;
                        let lat_ok  = fix.lat.map(|v| v.is_finite()).unwrap_or(false);
                        let lon_ok  = fix.lon.map(|v| v.is_finite()).unwrap_or(false);

                        if has_fix && lat_ok && lon_ok {
                            // Speed from gpsd is in m/s — convert to mph
                            let speed_mph = fix.speed
                                .filter(|v| v.is_finite())
                                .map(|v| (v * 2.23694) as f32)
                                .unwrap_or(0.0);

                            let heading_deg = fix.track.filter(|v| v.is_finite());

                            let altitude_m = fix.alt
                                .filter(|v| v.is_finite())
                                .unwrap_or(0.0);

                            let timestamp_s = fix.time
                                .map(|t| t as f64)
                                .unwrap_or(0.0);

                            *state.lock().unwrap() = GpsData {
                                lat:         fix.lat.unwrap(),
                                lon:         fix.lon.unwrap(),
                                altitude_m,
                                speed_mph,
                                heading_deg,
                                timestamp_s,
                                fix_valid:   true,
                                fix_mode,
                            };
                        }
                    }
                }
                Ok(None) => {
                    // No message yet — wait a bit (mirrors gps_rec.waiting(5000000))
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("[GPS] Read error: {e}. Reconnecting...");
                    break; // drop client, outer loop reconnects
                }
            }

            // mirrors sleep(1)
            thread::sleep(Duration::from_secs(1));
        }
    }
}

#[cfg(not(feature = "gps"))]
pub fn gps_main(state: SharedGpsState) {
    println!("[GPS] gpsd support not compiled in — GPS state will remain default.");
    *state.lock().unwrap() = GpsData::default();
}

pub fn spawn(state: SharedGpsState) {
    let s = Arc::clone(&state);
    thread::spawn(move || gps_main(s));
}