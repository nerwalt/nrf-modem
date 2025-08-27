#![no_std]
#![no_main]
#![macro_use]
#![allow(unused)]

use defmt::{debug, error, info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer, with_timeout};
use futures::StreamExt;
use nrf_modem::{self, GnssData, nrfxlib_sys};

use panic_probe as _;
use tinyrlibc as _;

use nrf_modem_examples::*;
use nrf_modem_examples::nmea_utils::*;

/// GPS Fix
#[derive(Debug, Default, Clone, Copy, defmt::Format)]
pub struct GpsFix {
    pub lat: f64,
    pub long: f64,
}

#[embassy_executor::main]
async fn main(_s: Spawner) {
    let _p = embassy_nrf::init(Default::default());

    info!("GPS Example");

    info!("Enable interrupts");
    enable_interrupts();

    info!("Initialize the modem");
    initialize_modem().await;

    // Perform a factory reset of the modem. This can be useful during development at times
    send_at!("AT%XFACTORYRESET=0");

    // Query hardware info
    send_at!("+CGMI");
    send_at!("+CGMM");
    send_at!("+CGMR");
    send_at!("+CGSN=1");
    send_at!("%SHORTSWVER");
    send_at!("%HWVERSION");
    send_at!("%XMODEMUUID");
    send_at!("%2DID");
    send_at!("%DEVICEUUID");

    send_at!("%XSYSTEMMODE?");

    // Power on
    info!("Power on modem");
    send_at!("+CFUN=1");
    Timer::after_secs(1).await;
    send_at!("+CFUN?");

    // Query ICCID
    info!("Sim Info");
    send_at!("+CIMI");
    send_at!("%XICCID");

    // Connect to the network
    // info!("Creating link");
    // let res = nrf_modem::LteLink::new().await;
    // if let Err(_) = res {
    //     error!("Error creating link");
    //     return
    // }
    // let link = res.unwrap();
    // info!("Connecting ... can take minutes");

    // let fut = link.wait_for_link();
    // if let Ok(_) = with_timeout(Duration::from_secs(60 * 5), fut).await {
    //     info!("Connected");
    // } else {
    //     error!("NOT Connected");
    // }
    
    // Query tac, eci, mcc, and mnc (data needed for cell-based locationing). This data is needed
    // for the assistance data query
    // send_at!("AT+CEREG=2");
    // let rsp = nrf_modem::send_at::<64>("AT+CEREG?").await.unwrap();
    // let (_, _, tac, eci, _) = parse_cereg(rsp.as_str()).unwrap();
    // info!("tac: {}  eci: {}", tac, eci);
    // let rsp = nrf_modem::send_at::<64>("AT+COPS?").await.unwrap();
    // let (_, _, plmn, _) = parse_cops(rsp.as_str()).unwrap();
    // let (mcc, mnc) = get_plmn_mcc_mnc(plmn).unwrap();
    // info!("mcc: {}  mnc: {}", mcc, mnc);

    let config = nrf_modem::GnssConfig {
        elevation_threshold_angle: 5,
        use_case: nrf_modem::GnssUsecase {
            low_accuracy: true,
            scheduled_downloads_disable: false,
        },
        nmea_mask: nrf_modem::NmeaMask {
            gga: true,
            gll: true,
            gsa: true,
            gsv: true,
            rmc: true,
        },
        timing_source: nrf_modem::GnssTimingSource::Rtc,
        // The following only affects continuous navigation mode, not one shot fixes
        power_mode: nrf_modem::GnssPowerSaveMode::Disabled,
    };

    let gnss = nrf_modem::Gnss::new().await.unwrap();

    send_at!("%XGNSSSTATUS");

    match nrf_modem::gnss::nrf_modem_gnss_agnss_data_expiry_get() {
        Ok(expiry_data) => {
            defmt::info!("A-GPS data expiry");
            defmt::info!(" data_flags: {:?}", expiry_data.data_flags);
            defmt::info!(" utc_expiry: {:?}", expiry_data.utc_expiry);
            defmt::info!(" klob_expiry: {:?}", expiry_data.klob_expiry);
            defmt::info!(" neq_expiry: {:?}", expiry_data.neq_expiry);
            defmt::info!(" integrity_expiry: {:?}", expiry_data.integrity_expiry);
            defmt::info!(" position_expiry: {:?}", expiry_data.position_expiry);
            defmt::info!(" sv_count: {:?}", expiry_data.sv_count);
        }
        Err(e) => {
            defmt::error!("Failed to get A-GPS expiry data");
        }
    }

    // Load assistance data
    static UTC_PARAMS: &[u8] = include_bytes!("../../assistance_data/utc_params.bin");
    static EPHEMERIDES: &[u8] = include_bytes!("../../assistance_data/ephemerides.bin");
    static ALMANAC: &[u8] = include_bytes!("../../assistance_data/almanac.bin");
    static KLOB_CORR: &[u8] = include_bytes!("../../assistance_data/klobuchar_ionspheric_corrections.bin");
    static SYSTEM_CLOCK_AND_TOWS: &[u8] = include_bytes!("../../assistance_data/system_clock.bin");
    static LOCATION: &[u8] = include_bytes!("../../assistance_data/location.bin");
    static INTEGRITY: &[u8] = include_bytes!("../../assistance_data/integrity.bin");
    unsafe {
        let p = UTC_PARAMS.as_ptr() as *mut core::ffi::c_void;
        let len = UTC_PARAMS.len() as i32;
        unsafe {
            nrfxlib_sys::nrf_modem_gnss_agnss_write(
                p, len, nrfxlib_sys::NRF_MODEM_GNSS_AGNSS_GPS_UTC_PARAMETERS as u16);
        }

        let p = EPHEMERIDES.as_ptr() as *mut core::ffi::c_void;
        let len = EPHEMERIDES.len() as i32;
        unsafe {
            nrfxlib_sys::nrf_modem_gnss_agnss_write(
                p, len, nrfxlib_sys::NRF_MODEM_GNSS_AGNSS_GPS_EPHEMERIDES as u16);
        }

        let p = ALMANAC.as_ptr() as *mut core::ffi::c_void;
        let len = ALMANAC.len() as i32;
        unsafe {
            nrfxlib_sys::nrf_modem_gnss_agnss_write(
                p, len, nrfxlib_sys::NRF_MODEM_GNSS_AGNSS_GPS_ALMANAC as u16);
        }

        let p = KLOB_CORR.as_ptr() as *mut core::ffi::c_void;
        let len = KLOB_CORR.len() as i32;
        unsafe {
            nrfxlib_sys::nrf_modem_gnss_agnss_write(
                p, len, nrfxlib_sys::NRF_MODEM_GNSS_AGNSS_KLOBUCHAR_IONOSPHERIC_CORRECTION as u16);
        }

        let p = SYSTEM_CLOCK_AND_TOWS.as_ptr() as *mut core::ffi::c_void;
        let len = SYSTEM_CLOCK_AND_TOWS.len() as i32;
        unsafe {
            nrfxlib_sys::nrf_modem_gnss_agnss_write(
                p, len, nrfxlib_sys::NRF_MODEM_GNSS_AGNSS_GPS_SYSTEM_CLOCK_AND_TOWS as u16);
        }

        let p = LOCATION.as_ptr() as *mut core::ffi::c_void;
        let len = LOCATION.len() as i32;
        unsafe {
            nrfxlib_sys::nrf_modem_gnss_agnss_write(
                p, len, nrfxlib_sys::NRF_MODEM_GNSS_AGNSS_LOCATION as u16);
        }

        // Use AGNSS INTEGRITY instead of AGPS INTEGRITY on modem firmware >= 2. The latter is an
        // older model for older modem firmware verisons.
        let p = INTEGRITY.as_ptr() as *mut core::ffi::c_void;
        let len = INTEGRITY.len() as i32;
        unsafe {
            nrfxlib_sys::nrf_modem_gnss_agnss_write(
                p, len, nrfxlib_sys::NRF_MODEM_GNSS_AGNSS_INTEGRITY as u16);
        }
    }

    match nrf_modem::gnss::nrf_modem_gnss_agnss_data_expiry_get() {
        Ok(expiry_data) => {
            defmt::info!("A-GPS data expiry");
            defmt::info!(" data_flags: {:?}", expiry_data.data_flags);
            defmt::info!(" utc_expiry: {:?}", expiry_data.utc_expiry);
            defmt::info!(" klob_expiry: {:?}", expiry_data.klob_expiry);
            defmt::info!(" neq_expiry: {:?}", expiry_data.neq_expiry);
            defmt::info!(" integrity_expiry: {:?}", expiry_data.integrity_expiry);
            defmt::info!(" position_expiry: {:?}", expiry_data.position_expiry);
            defmt::info!(" sv_count: {:?}", expiry_data.sv_count);
        }
        Err(e) => {
            defmt::error!("Failed to get A-GPS expiry data");
        }
    }

    const TIMEOUT: u16 = 60*30;
    let mut gnss_stream = gnss.start_single_fix(config, TIMEOUT).unwrap();
    // let mut gnss_stream = gnss.start_continuous_fix(config).unwrap();

    // Monitor stream for a fix
    loop {
        match gnss_stream.next().await {
            Some(Ok(GnssData::PositionVelocityTime(pvt))) => {
                let flags = PvtFlags::from_u8(pvt.flags);
                info!("Pvt -> utc: {}:{}:{}.{} latitude: {} longitude: {} flags: {} {:?}",
                    pvt.datetime.hour,
                    pvt.datetime.minute,
                    pvt.datetime.seconds,
                    pvt.datetime.ms,
                    pvt.latitude,
                    pvt.longitude,
                    pvt.flags,
                    flags,
                    );


                // Check if we have a fix
                if (pvt.flags & nrf_modem::nrfxlib_sys::NRF_MODEM_GNSS_PVT_FLAG_FIX_VALID as u8) != 0 {
                    let fix = GpsFix {
                        lat: pvt.latitude,
                        long: pvt.longitude,
                    };
                    info!("GpsFix -> utc: {:?}", fix);
                    break;
                }
            },
            Some(Ok(GnssData::Nmea(nmea))) => {
                if let Some(res) = parse_nmea(&nmea) {
                    info!("Nmea -> {:?}", res);
                } else {
                    // info!("Nmea -> {:?}", nmea.as_str());
                }
            },
            Some(Ok(GnssData::Agps(_agps))) => {
                info!("Agps -> ");
            },
            None => {
                warn!("Timeout");
                break;
            },
            _ => {
                error!("I don't know what happened");
                break;
            }
        }
    }

    // Manually free the GNSS stream and deactivate the GNSS, so that we
    // do so asynchronously
    let gnss = gnss_stream.free();
    gnss.deactivate().await.unwrap();

    info!("Done!");
}

