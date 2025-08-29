#![no_std]
#![no_main]
#![allow(unused)]

use defmt::{error, info};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_time::{Timer, Duration, with_timeout};
use nrf_modem;
// use nrf_modem::no_std_net::{IpAddr, Ipv4Addr, SocketAddr};
use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};
use panic_probe as _;
use tinyrlibc as _;

use rust_mqtt::{
    client::{client::MqttClient, client_config::ClientConfig},
    packet::v5::publish_packet::QualityOfService::{QoS0, QoS1},
    utils::rng_generator::CountingRng,
};


use embedded_io_async::Write;

use nrf_modem_examples::csm_utils::*;
use nrf_modem_examples::*;

const SECURITY_TAG: u32 = 1;

// const SERVER_URL: &str = "qptag-ota.trace-eng.com";
const SERVER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(54, 184, 63, 61));
const SERVER_PORT: u16 = 10443;

#[cfg(feature = "install_creds")]
const ROOTCA_CERT: &str =
    // include_str!("/home/tlawren/work/projects/tls_testing/certs/local/ec/rootCA.crt");
    include_str!("/home/tlawren/work/aws/test_creds/AmazonRootCA3.pem");

#[cfg(feature = "install_creds")]
const CLIENT_CERT: &str =
    // include_str!("/home/tlawren/work/projects/tls_testing/certs/local/ec/client.crt");
    include_str!("/home/tlawren/work/aws/test_creds/49f23c42134d516fd2940bd51b3b108f1bc96f73d1e6ab38793e42476dbdff72-certificate.pem.crt");

#[cfg(feature = "install_creds")]
const CLIENT_KEY: &str =
    // include_str!("/home/tlawren/work/projects/tls_testing/certs/local/ec/client.key");
    include_str!("/home/tlawren/work/aws/test_creds/49f23c42134d516fd2940bd51b3b108f1bc96f73d1e6ab38793e42476dbdff72-private.pem.key");

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = embassy_nrf::init(Default::default());

    info!("TLS Example");

    enable_interrupts();
    initialize_modem().await;

    // Perform a factory reset of the modem. This can be useful during development at times
    // info!("Factory Reset");
    // let response = nrf_modem::send_at::<64>("AT%XFACTORYRESET=0").await.unwrap();

    #[cfg(feature = "install_creds")]
    {
        info!("Updating security credentials");
        // The modem has to be off to update credentials
        let _ = nrf_modem::send_at::<64>("AT+CFUN=0").await.unwrap();
        Timer::after_secs(1).await;
        key_delete(SECURITY_TAG, CSMType::RootCert).await.unwrap();
        key_delete(SECURITY_TAG, CSMType::ClientCert).await.unwrap();
        key_delete(SECURITY_TAG, CSMType::ClientPrivateKey)
            .await
            .unwrap();
        Timer::after_secs(1).await;
        key_write(SECURITY_TAG, CSMType::RootCert, ROOTCA_CERT)
            .await
            .unwrap();
        key_write(SECURITY_TAG, CSMType::ClientCert, CLIENT_CERT)
            .await
            .unwrap();
        key_write(SECURITY_TAG, CSMType::ClientPrivateKey, CLIENT_KEY)
            .await
            .unwrap();
    }

    // Power on
    info!("Power on modem");
    nrf_modem::send_at::<64>("AT+CFUN=1").await.unwrap();

    // Create link and connect to internet
    info!("Creating link");
    let link = nrf_modem::LteLink::new().await.unwrap();
    let timeout = Duration::from_secs(3*60);
    info!("Connecting ... waiting up to {} s for connection", timeout.as_secs());
    let fut = link.wait_for_link();
    match with_timeout(timeout, fut).await {
        Ok(_) => info!("Connected"),
        Err(_) => {
            error!("Timeout waiting for connection");
            return;
        }
    }

    // DNS look the server ip
    let url = "a25eiq2paa6ou3-ats.iot.us-east-2.amazonaws.com";
    // let ip = nrf_modem::get_host_by_name(url).await.unwrap();
    // match ip {
    //     IpAddr::V4(addr) => {
    //         let parts = addr.octets();
    //         info!(" {}.{}.{}.{}", parts[0], parts[1], parts[2], parts[3]);
    //     }
    //     _ => {
    //         error!("Failed to get server IP");
    //     }
    // }
    // let addr = SocketAddr::from((ip, 8833));

    // let addr = SocketAddr::from((SERVER_IP, SERVER_PORT));

    // Connect to server
    let timeout = Duration::from_secs(30);
    info!("Connecting TLS stream ... waiting up to {} s for a connection", timeout.as_secs());
    let fut = nrf_modem::TlsStream::connect(
        url,
        8883,
        nrf_modem::PeerVerification::Optional,
        &[SECURITY_TAG],
        None,
    );
    let res = match with_timeout(timeout, fut).await {
        Ok(res) => res,
        Err(err) => {
            error!("Timeout connecting to TLS stream");
            return
        }
    };
    let mut stream = match res {
        Ok(stream) => {
            info!("Connected TLS stream");
            stream
        }
        Err(err) => {
            error!("Error connecting TLS stream: {:?}", err);
            return
        }
    };

    // Define your topic and message as byte slices.
    const TOPIC: &[u8] = b"helloWorld";
    const PAYLOAD: &[u8] = b"hello world";

    let mut config = ClientConfig::new(
        rust_mqtt::client::client_config::MqttVersion::MQTTv5,
        CountingRng(20000),
    );
    config.add_max_subscribe_qos(QoS0);
    config.add_client_id("123456789");
    const RW_BUFFER_SIZE: usize = 4096;
    config.max_packet_size = RW_BUFFER_SIZE as u32;;
    let mut recv_buffer = [0; RW_BUFFER_SIZE];
    let mut write_buffer = [0; RW_BUFFER_SIZE];

    let mut client =
        MqttClient::<_, 5, _>::new(stream, &mut write_buffer, RW_BUFFER_SIZE, &mut recv_buffer, RW_BUFFER_SIZE, config);

    info!("->1");
    let res = client.connect_to_broker().await;
    match res {
        Ok(_) => {
            info!("Connected to MQTT broker");
        }
        Err(err) => {
            error!("Error connecting to MQTT broker: {:?}", err);
            return
        }
    };


    info!("->2");
    client.send_message("dt/helloWorld", PAYLOAD, rust_mqtt::packet::v5::publish_packet::QualityOfService::QoS0, false).await;

    info!("->3");

    // // Calculate lengths.
    // let topic_len = TOPIC.len();
    // let payload_len = PAYLOAD.len();
    // let remaining_length = 2 + topic_len + payload_len; // 2 bytes for topic length

    // // Allocate a fixed-size buffer.
    // let mut packet: [u8; 128] = [0; 128];
    // let mut pos = 0;

    // // Fixed header: 0x30 indicates PUBLISH, QoS 0.
    // packet[pos] = 0x30;
    // pos += 1;

    // // Remaining length (assume <128 for simplicity, so one byte is enough).
    // packet[pos] = remaining_length as u8;
    // pos += 1;

    // // Variable header: write topic length (big-endian).
    // packet[pos] = (topic_len >> 8) as u8;
    // pos += 1;
    // packet[pos] = (topic_len & 0xFF) as u8;
    // pos += 1;

    // // Write the topic.
    // for &b in TOPIC {
    //     packet[pos] = b;
    //     pos += 1;
    // }

    // // Write the payload.
    // for &b in PAYLOAD {
    //     packet[pos] = b;
    //     pos += 1;
    // }

    // match stream.write(&packet[..pos]).await {
    //     Ok(_) => {
    //         info!("Published MQTT messgae");
    //     }
    //     Err(err) => {
    //         error!("Error publishing to MQTT topic");
    //         return
    //     }
    // }

    // Timer::after_secs(1).await;

    // let msgs = ["dead", "beef", "is", "freaking", "amazing"];
    // let mut buf = [0u8; 32];
    // for msg in msgs {
    //     info!("Writing: {}", msg);
    //     let _ = stream.write(msg.as_bytes()).await;
    //     let rsp = stream.receive(&mut buf).await;
    //     if let Ok(rsp) = rsp {
    //         let received_msg = core::str::from_utf8(&rsp).unwrap_or("[Invalid UTF-8]");
    //         info!("Received: {}", received_msg);
    //     } else {
    //         info!("Error receiving data: {:?}", rsp);
    //     }
    //     Timer::after_secs(1).await;
    // }

    // Timer::after_secs(1).await;

    // // Now use an adapted stream
    // // Adapt to embedded_io_async Read + Write
    // let mut stream = TlsStreamAdapter::new(stream);

    // Timer::after_secs(1).await;

    // let msgs = ["so", "is", "dead", "ducks"];
    // let mut buf = [0u8; 32];
    // for msg in msgs {
    //     info!("Writing: {}", msg);
    //     let _ = stream.write(msg.as_bytes()).await;
    //     let res = stream.read(&mut buf).await;
    //     if let Ok(res) = res {
    //         let received_msg = core::str::from_utf8(&buf[..res]).unwrap_or("[Invalid UTF-8]");
    //         info!("Received: {}", received_msg);
    //     } else {
    //         info!("Error receiving data");
    //     }
    //     Timer::after_secs(1).await;
    // }

    // let _ = link.deactivate().await;

    info!("Done!");
}
