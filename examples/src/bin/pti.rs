#![no_std]
#![no_main]

use defmt::{debug, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use nrf_modem;
use panic_probe as _;
use tinyrlibc as _;

use nrf_modem_examples::*;

macro_rules! send_at {
    ($cmd:expr) => {
        let response = nrf_modem::send_at::<64>(concat!("AT", $cmd)).await.unwrap();
        defmt::debug!("{}: {:?}", $cmd, response.as_str());
    };
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_nrf::init(Default::default());

    info!("PTI Example");

    info!("Enable interrupts");

    enable_interrupts();
    let mode = nrf_modem::SystemMode {
        lte_support: true,
        lte_psm_support: true,
        nbiot_support: false,
        gnss_support: true,
        preference: nrf_modem::ConnectionPreference::Lte,
        pti_support: true,
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

    send_at!("+CGMI");
    send_at!("+CGMM");
    send_at!("+CGMR");
    send_at!("+CGSN");
    send_at!("%SHORTSWVER");
    send_at!("%HWVERSION");
    send_at!("%XMODEMUUID");
    send_at!("%2DID");
    send_at!("%DEVICEUUID");

    send_at!("%XVBAT");

    send_at!("%POWERCLASS?");

    send_at!("%XSYSTEMMODE=1,0,0,0");
    send_at!("%XRFTEST=2,1,-80,0");

    Timer::after_secs(3).await;

    // info!("Setting Power Class");
    // send_at!("__%POWERCLASS=3");

    // Timer::after(Duration::from_secs(1)).await;
    // info!("Setting IEMI");
    // send_at!("__%IMEIWRITE=0,\"358757450004257\"");

    // Timer::after(Duration::from_secs(1)).await;
    // info!("Fsyncing");
    // send_at!("__%XFSSYNC");

    info!("Done!");
}
