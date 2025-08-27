//! nRF91 modem Credential Storage Management Utilities
//!

use arrayvec::ArrayString;

/// Credential Storage Management Types
#[derive(Clone, Copy, Debug, defmt::Format)]
#[allow(dead_code)]
pub enum CSMType {
    RootCert = 0,
    ClientCert = 1,
    ClientPrivateKey = 2,
    Psk = 3,
    PskId = 4,
    PublicKey = 5,
    Reserved = 6,
}

/// Write a key for the given security tag to the modem
pub async fn key_write(sec_tag: u32, ty: CSMType, data: &str) -> Result<(), nrf_modem::Error> {
    let mut cmd: ArrayString<2048> = ArrayString::new();
    core::fmt::write(
        &mut cmd,
        format_args!(r#"AT%CMNG=0,{},{},"{}""#, sec_tag, ty as u32, data),
    )
    .unwrap();
    let _rsp = nrf_modem::send_at::<32>(cmd.as_str()).await?;
    // defmt::debug!("key_write: {} {} {}", sec_tag, ty, _rsp.as_str());
    Ok(())
}

/// List a key for the given security tag in the modem
pub async fn key_list(sec_tag: u32, ty: CSMType) -> Result<(), nrf_modem::Error> {
    let mut cmd: ArrayString<2048> = ArrayString::new();
    core::fmt::write(
        &mut cmd,
        format_args!(r#"AT%CMNG=1,{},{}"#, sec_tag, ty as u32),
    )
    .unwrap();
    let _rsp = nrf_modem::send_at::<32>(cmd.as_str()).await?;
    // defmt::debug!("key_list: {} {} {}", sec_tag, ty, _rsp.as_str());
    Ok(())
}

/// Read a key for the given security tag from the modem
pub async fn key_read(sec_tag: u32, ty: CSMType) -> Result<(), nrf_modem::Error> {
    let mut cmd: ArrayString<32> = ArrayString::new();
    core::fmt::write(
        &mut cmd,
        format_args!("AT%CMNG=2,{},{}", sec_tag, ty as u32),
    )
    .unwrap();
    let _rsp = nrf_modem::send_at::<32>(cmd.as_str()).await?;
    // defmt::debug!("key_delete: {} {} {}", sec_tag, ty, _rsp.as_str());
    Ok(())
}


/// Delete a key for the given security tag from the modem
pub async fn key_delete(sec_tag: u32, ty: CSMType) -> Result<(), nrf_modem::Error> {
    let mut cmd: ArrayString<32> = ArrayString::new();
    core::fmt::write(
        &mut cmd,
        format_args!("AT%CMNG=3,{},{}", sec_tag, ty as u32),
    )
    .unwrap();
    let _rsp = nrf_modem::send_at::<32>(cmd.as_str()).await?;
    // defmt::debug!("key_delete: {} {} {}", sec_tag, ty, _rsp.as_str());
    Ok(())
}

