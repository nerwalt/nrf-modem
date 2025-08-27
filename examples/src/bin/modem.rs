#![no_std]
#![no_main]

use defmt::{error, debug, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Timer, Duration, with_timeout};
use nrf_modem;
use core::net::{IpAddr, Ipv4Addr, SocketAddr};
// use nrf_modem::no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
use panic_probe as _;
use tinyrlibc as _;
use embassy_nrf::gpio::{Level, Output, OutputDrive};

use nrf_modem_examples::*;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    info!("Modem Example");

    info!("Enable interrupts");
    enable_interrupts();

    info!("Initialize the modem");
    initialize_modem().await;

    // Perform a factory reset of the modem. This can be useful during development at times
    // info!("Factory Reset");
    // let response = nrf_modem::send_at::<64>("AT%XFACTORYRESET=0").await.unwrap();
    // info!(" {:?}", response.as_str());

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

    // send_at!("%XSYSTEMMODE?");

    // Power on
    info!("Power on modem");
    send_at!("+CFUN=1");
    Timer::after(Duration::from_secs(1)).await;
    send_at!("+CFUN?");

    // Query ICCID
    info!("Sim Info");
    send_at!("+CIMI");
    send_at!("%XICCID");

    // Connect
    info!("Creating link");
    let link = nrf_modem::LteLink::new().await.unwrap();
    let timeout = Duration::from_secs(3*60);
    info!("Connecting ... (Waiting up to {} s for connection)", timeout.as_secs());
    let fut = link.wait_for_link();
    match with_timeout(timeout, fut).await {
        Ok(_) => info!("Connected"),
        Err(_) => {
            error!("Timeout waiting for connection");
            return;
        }
    }

    // Look up Google IP
    let google_ip = nrf_modem::get_host_by_name("www.google.com").await.unwrap();
    match google_ip {
        IpAddr::V4(addr) => {
            let parts = addr.octets();
            info!(" {}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);
        }
        _ => {}
    }

    // // TCP example
    // let stream = nrf_modem::TcpStream::connect(SocketAddr::from((google_ip, 80)))
    //     .await
    //     .unwrap();

    // stream
    //     .write("GET / HTTP/1.0\nHost: google.com\r\n\r\n".as_bytes())
    //     .await
    //     .unwrap();

    // let mut buffer = [0; 1024];
    // let received = stream.receive(&mut buffer).await.unwrap();

    // info!(
    //     "Google response: {}",
    //     core::str::from_utf8(received).unwrap()
    // );

    // // Drop the stream async (normal Drop is ok too, but that's blocking)
    // stream.deactivate().await.unwrap();

    // // Create socket
    // let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 53);
    // let socket = nrf_modem::UdpSocket::bind(addr).await.unwrap();
    // info!("Socket created");

    // // Do a DNS request
    // let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53);
    // socket
    //     .send_to(
    //         &[
    //             0xdb, 0x42, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x77,
    //             0x77, 0x77, 0x0C, 0x6E, 0x6F, 0x72, 0x74, 0x68, 0x65, 0x61, 0x73, 0x74, 0x65, 0x72,
    //             0x6E, 0x03, 0x65, 0x64, 0x75, 0x00, 0x00, 0x01, 0x00, 0x01,
    //         ],
    //         addr,
    //     )
    //     .await
    //     .unwrap();

    // info!("DNS request sent");

    // let mut buffer = [0u8; 1024];
    // let (response, source_addr) = socket.receive_from(&mut buffer).await.unwrap();
    // info!(" {:?}", response);
    // match source_addr.ip() {
    //     IpAddr::V4(addr) => {
    //         let parts = addr.octets();
    //         info!(" {}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);
    //     }
    //     _ => {}
    // }

    // let _ = link.deactivate().await;
    
    // // Query tac, eci, mcc, and mnc (data needed for cell-based locationing). This data is needed
    // // for the assistance data query
    // send_at!("AT+CEREG=2");
    // let rsp = nrf_modem::send_at::<64>("AT+CEREG?").await.unwrap();
    // let (_, _, tac, eci, _) = parse_cereg(rsp.as_str()).unwrap();
    // info!("tac: {}  eci: {}", tac, eci);
    // let rsp = nrf_modem::send_at::<64>("AT+COPS?").await.unwrap();
    // let (_, _, plmn, _) = parse_cops(rsp.as_str()).unwrap();
    // let (mcc, mnc) = get_plmn_mcc_mnc(plmn).unwrap();
    // info!("mcc: {}  mnc: {}", mcc, mnc);

    info!("Done!");
}
