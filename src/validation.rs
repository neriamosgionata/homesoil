use std::net::IpAddr;

/// Validate an IP address string. Must be valid IPv4 or IPv6.
/// In non-dev mode, rejects loopback addresses.
pub fn validate_ip_address(ip: &str) -> Result<(), String> {
    let addr: IpAddr = ip
        .parse()
        .map_err(|_| format!("Invalid IP address: {}", ip))?;

    let is_dev = std::env::var("IS_DEV").is_ok();

    if !is_dev && addr.is_loopback() {
        return Err(format!("Loopback address not allowed: {}", ip));
    }

    Ok(())
}

/// Validate a port number. Must be within the i16 range and non-zero.
pub fn validate_port(port: i16) -> Result<(), String> {
    if port <= 0 {
        return Err(format!("Port out of valid range (1-32767): {}", port));
    }
    Ok(())
}

/// Validate a sensor value is a parseable number.
pub fn validate_sensor_value(value: &str) -> Result<(), String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("Sensor value not a valid number: {}", value))?;
    Ok(())
}

/// Validate payload size doesn't exceed limit (1KB).
pub fn validate_payload_size(payload: &str, max_bytes: usize) -> Result<(), String> {
    if payload.len() > max_bytes {
        return Err(format!(
            "Payload too large: {} bytes (max {})",
            payload.len(),
            max_bytes
        ));
    }
    Ok(())
}
