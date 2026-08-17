use std::fmt;

#[derive(Debug)]
pub enum Error {
    Usb(String),
    Status(u16),
    SignatureValidationFailed(u16),
    PairingFailed,
    Flash(String),
    Crypto(String),
    Io(std::io::Error),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Usb(s) => write!(f, "USB error: {}", s),
            Error::Status(s) => write!(f, "Sensor returned error status 0x{:04x}", s),
            Error::SignatureValidationFailed(s) => {
                write!(f, "Signature validation failed: 0x{:04x}", s)
            }
            Error::PairingFailed => write!(
                f,
                "Signature verification failed. This device was probably paired with another \
                 computer (e.g. by the Windows Hello driver). Re-pair it with: omnilock-bio reprovision"
            ),
            Error::Flash(s) => write!(f, "Flash error: {}", s),
            Error::Crypto(s) => write!(f, "Crypto error: {}", s),
            Error::Io(e) => write!(f, "I/O error: {}", e),
            Error::Other(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

pub fn assert_status(rsp: &[u8]) -> Result<()> {
    if rsp.len() < 2 {
        return Err(Error::Other("response shorter than 2 bytes".into()));
    }
    let s = u16::from_le_bytes([rsp[0], rsp[1]]);
    if s != 0 {
        if s == 0x44f {
            return Err(Error::SignatureValidationFailed(s));
        }
        return Err(Error::Status(s));
    }
    Ok(())
}