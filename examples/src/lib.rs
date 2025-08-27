#![cfg_attr(not(test), no_std)]

pub mod csm_utils;
pub mod nmea_utils;

use nrf_modem::nrfxlib_sys;

use embassy_nrf::{interrupt, interrupt::InterruptExt};
// use arrayvec::ArrayString;

// Helper to send an AT command and print its response
#[macro_export]
macro_rules! send_at {
    ($cmd:expr) => {
        let response = nrf_modem::send_at::<64>(concat!("AT", $cmd)).await.unwrap();
        debug!("{}: {:?}", $cmd, response.as_str());
    };
}

// Bind the required interrupts
#[interrupt]
#[allow(non_snake_case)]
fn EGU1() {
    nrf_modem::ipc_irq_handler();
    cortex_m::asm::sev();
}

#[interrupt]
#[allow(non_snake_case)]
fn IPC() {
    nrf_modem::ipc_irq_handler();
    cortex_m::asm::sev();
}

// Enable interrupts.
pub fn enable_interrupts() {
    interrupt::EGU1.set_priority(interrupt::Priority::P4);
    interrupt::IPC.set_priority(interrupt::Priority::P1);
    unsafe {
        interrupt::EGU1.enable();
        interrupt::IPC.enable();
    }
}

// Initialize the modem
pub async fn initialize_modem() {
    let mode = nrf_modem::SystemMode {
        lte_support: true,
        lte_psm_support: true,
        nbiot_support: false,
        gnss_support: true,
        preference: nrf_modem::ConnectionPreference::Lte,
        // pti_support: false,
    };

    #[cfg(feature = "nrf9160")]
    nrf_modem::init(mode).await.unwrap();

    #[cfg(feature = "nrf9151")]
    let memory_layout = nrf_modem::MemoryLayout {
        base_address: 0x2000_8000,
        tx_area_size: 0x2080,
        rx_area_size: 0x2000,
        // Trace is not implemented yet
        trace_area_size: 0,
    };

    #[cfg(feature = "nrf9151")]
    nrf_modem::init_with_custom_layout(mode, memory_layout)
        .await
        .unwrap();
}

/// Parse an XICCID AT command response
pub fn parse_xiccid(response: &str) -> Result<&str, ()> {
    let (iccid,) = at_commands::parser::CommandParser::parse(response.as_bytes())
        .expect_identifier(b"%XICCID:")
        .expect_raw_string()
        .finish()
        .map_err(|_| ())?;
    // exclude the last character, which is a checksum
    let iccid = &iccid[..iccid.len() - 1];
    Ok(iccid)
}

/// Parse an CGPADDR AT command response
pub fn parse_cgpaddr(response: &str) -> Result<&str, ()> {
    let (ip_address,) = at_commands::parser::CommandParser::parse(response.as_bytes())
        .expect_identifier(b"+CGPADDR:")
        .expect_raw_string()
        .finish()
        .map_err(|_| ())?;
    // The IP address is returned as a string in the format "+CGPADDR: <cid>,"<ip_address>"
    // We can trim any surrounding quotes or unwanted characters.
    let ip_address = ip_address.trim_matches('"');
    Ok(ip_address)
}

/// Parse a CEREG AT command response
pub fn parse_cereg(response: &str) -> Result<(i32, i32, u32, u32, i32), ()> {
    let (n, stat, tac, eci, act) = at_commands::parser::CommandParser::parse(response.as_bytes())
        .expect_identifier(b"+CEREG:")
        .expect_int_parameter()
        .expect_int_parameter()
        .expect_string_parameter()
        .expect_string_parameter()
        .expect_int_parameter()
        .finish()
        .map_err(|_| ())?;

    let tac = u32::from_str_radix(tac, 16).map_err(|_| ())?;

    let eci = u32::from_str_radix(eci, 16).map_err(|_| ())?;

    Ok((n, stat, tac, eci, act))
}

/// Parse a COPS AT command response
pub fn parse_cops(response: &str) -> Result<(i32, i32, &str, i32), ()> {
    // Note, oper == plmn
    let (mode, format, plmn, act) = at_commands::parser::CommandParser::parse(response.as_bytes())
        .expect_identifier(b"+COPS:")
        .expect_int_parameter()
        .expect_int_parameter()
        .expect_string_parameter()
        .expect_int_parameter()
        .finish()
        .map_err(|_| ())?;

    Ok((mode, format, plmn, act))
}

/// Get the MCC and MNC numbers out of the PLMN string
pub fn get_plmn_mcc_mnc(plmn: &str) -> Result<(u32, u32), ()> {
    let mcc = plmn.get(0..3).unwrap_or("");
    let mnc = plmn.get(3..).unwrap_or("");

    let mcc = mcc.parse::<u32>().map_err(|_| ())?;
    let mnc = mnc.parse::<u32>().map_err(|_| ())?;

    Ok((mcc, mnc))
}


// #[repr(u8)]
// #[derive(Debug, Clone, Copy)]
// pub struct PvtFlags(u8);

#[derive(Default, Debug, Clone, Copy)]
#[derive(defmt::Format)]
pub struct PvtFlags {
    fix_valid: bool, 
    leap_second_valid: bool, 
    sleep_between_pvt: bool, 
    deadline_missed: bool, 
    not_enough_window_time: bool, 
    velocit_valid: bool, 
    sched_download: bool, 
}

impl PvtFlags {
    // pub const FIX_VALID: u8 = 1;
    // pub const LEAP_SECOND_VALID: u8 = 2;
    // pub const SLEEP_BETWEEN_PVT: u8 = 4;
    // pub const DEADLINE_MISSED: u8 = 8;
    // pub const NOT_ENOUGH_WINDOW_TIME: u8 = 16;
    // pub const VELOCITY_VALID: u8 = 32;
    // pub const SCHED_DOWNLOAD: u8 = 64;

    /// Converts a `u8` flag byte into a `PvtFlags` struct.
    pub fn from_u8(flags: u8) -> Self {
        let mut s = Self::default();
        s.fix_valid = s.contains(flags, nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_FIX_VALID as u8);
        s.leap_second_valid = s.contains(flags, nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_LEAP_SECOND_VALID as u8);
        s.sleep_between_pvt = s.contains(flags, nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_SLEEP_BETWEEN_PVT as u8);
        s.deadline_missed = s.contains(flags, nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_DEADLINE_MISSED as u8);
        s.not_enough_window_time = s.contains(flags, nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_NOT_ENOUGH_WINDOW_TIME as u8);
        s.velocit_valid = s.contains(flags, nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_VELOCITY_VALID as u8);
        s.sched_download = s.contains(flags, nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_SCHED_DOWNLOAD as u8);
        s
    }

    /// Checks if a specific flag is set.
    pub fn contains(&self, value: u8, flag: u8) -> bool {
        (value & flag) != 0
    }

    // /// Returns true if a valid fix has been acquired.
    // pub fn has_fix(&self) -> bool {
    //     self.contains(nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_FIX_VALID)
    // }

    // /// Returns true if the velocity estimate is valid.
    // pub fn has_velocity(&self) -> bool {
    //     self.contains(nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_VELOICTY_VALID)
    // }
}

