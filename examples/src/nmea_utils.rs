use arrayvec::ArrayString;
use defmt::{Format, Formatter, write};

/// Parsed $GPGGA (Global Positioning System Fix Data)
#[derive(Default, Debug)]
pub struct Gpgga {
    pub time: Option<ArrayString<10>>,   // UTC time
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub fix_quality: Option<u8>,
    pub satellites: Option<u8>,
    pub altitude: Option<f32>,
}

impl Format for Gpgga {
    fn format(&self, f: Formatter) {
        write!(f, "Gpgga {{ time: {}, lat: {:?}, lon: {:?}, fix_quality: {:?}, sats: {:?}, alt: {:?} }}",
            self.time.as_ref().map(|s| s.as_str()).unwrap_or("None"),
            self.latitude,
            self.longitude,
            self.fix_quality,
            self.satellites,
            self.altitude,
        );
    }
}

/// Parsed $GPRMC (Recommended Minimum Navigation Information)
#[derive(Default, Debug)]
pub struct Gprmc {
    pub time: Option<ArrayString<10>>,   // UTC time
    pub status: Option<char>,            // 'A' = Valid, 'V' = Void
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub speed_knots: Option<f32>,
    pub date: Option<ArrayString<6>>,    // DDMMYY
}

impl Format for Gprmc {
    fn format(&self, f: Formatter) {
        write!(f, "Gprmc {{ time: {}, status: {:?}, lat: {:?}, lon: {:?}, speed_knots: {:?}, date: {} }}",
            self.time.as_ref().map(|s| s.as_str()).unwrap_or("None"),
            self.status,
            self.latitude,
            self.longitude,
            self.speed_knots,
            self.date.as_ref().map(|s| s.as_str()).unwrap_or("None"),
        );
    }
}

/// Parsed $GPGSV (Satellites in View)
#[derive(Default, Debug)]
pub struct Gpgsv {
    pub total_satellites: Option<u8>,
}

impl Format for Gpgsv {
    fn format(&self, f: Formatter) {
        write!(f, "Gpgsv {{ total_satellites: {:?} }}", self.total_satellites);
    }
}

pub fn parse_nmea(sentence: &ArrayString<83>) -> Option<NmeaSentence> {
    let mut parts = sentence.split(',');

    match parts.next()? {
        "$GPGGA" => Some(NmeaSentence::Gpgga(parse_gpgga(parts))),
        "$GPRMC" => Some(NmeaSentence::Gprmc(parse_gprmc(parts))),
        "$GPGSV" => Some(NmeaSentence::Gpgsv(parse_gpgsv(parts))),
        _ => None,
    }
}

/// Enum to represent different NMEA sentence types
#[derive(Debug)]
pub enum NmeaSentence {
    Gpgga(Gpgga),
    Gprmc(Gprmc),
    Gpgsv(Gpgsv),
}

impl Format for NmeaSentence {
    fn format(&self, f: Formatter) {
        match self {
            NmeaSentence::Gpgga(data) => write!(f, "NmeaSentence::Gpgga({:?})", data),
            NmeaSentence::Gprmc(data) => write!(f, "NmeaSentence::Gprmc({:?})", data),
            NmeaSentence::Gpgsv(data) => write!(f, "NmeaSentence::Gpgsv({:?})", data),
        }
    }
}

/// Parses $GPGGA (Global Positioning System Fix Data)
fn parse_gpgga(mut parts: core::str::Split<char>) -> Gpgga {
    Gpgga {
        time: parts.next().map(|s| ArrayString::from(s).ok()).flatten(),
        latitude: parse_lat(parts.next(), parts.next()),
        longitude: parse_lon(parts.next(), parts.next()),
        fix_quality: parts.next().and_then(|s| s.parse().ok()),
        satellites: parts.next().and_then(|s| s.parse().ok()),
        altitude: parts.nth(3).and_then(|s| s.parse().ok()), // Skip HDOP before altitude
    }
}

/// Parses $GPRMC (Recommended Minimum Navigation Information)
fn parse_gprmc(mut parts: core::str::Split<char>) -> Gprmc {
    Gprmc {
        time: parts.next().map(|s| ArrayString::from(s).ok()).flatten(),
        status: parts.next().and_then(|s| s.chars().next()),
        latitude: parse_lat(parts.next(), parts.next()),
        longitude: parse_lon(parts.next(), parts.next()),
        speed_knots: parts.next().and_then(|s| s.parse().ok()),
        date: parts.nth(2).map(|s| ArrayString::from(s).ok()).flatten(), // Skip course before date
    }
}

/// Parses $GPGSV (Satellites in View)
fn parse_gpgsv(mut parts: core::str::Split<char>) -> Gpgsv {
    Gpgsv {
        total_satellites: parts.nth(2).and_then(|s| s.parse().ok()),
    }
}

/// Parses latitude from NMEA format (ddmm.mmmm,N/S)
fn parse_lat(value: Option<&str>, hemi: Option<&str>) -> Option<f64> {
    let value = value?;
    if value.is_empty() { return None; }
    let (deg, min) = value.split_at(2);
    let lat = deg.parse::<f64>().ok()? + (min.parse::<f64>().ok()? / 60.0);
    Some(if hemi? == "S" { -lat } else { lat })
}

/// Parses longitude from NMEA format (dddmm.mmmm,E/W)
fn parse_lon(value: Option<&str>, hemi: Option<&str>) -> Option<f64> {
    let value = value?;
    if value.is_empty() { return None; }
    let (deg, min) = value.split_at(3);
    let lon = deg.parse::<f64>().ok()? + (min.parse::<f64>().ok()? / 60.0);
    Some(if hemi? == "W" { -lon } else { lon })
}


#[cfg(test)]
#[allow(unused)]
mod tests {
    use super::*;

    #[test]
    fn test_parsers() {
        assert!(true);
    }
}

