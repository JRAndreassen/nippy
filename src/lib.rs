/*!
# Example
Shows how to use the ntp library to fetch the current time according
to pool ntp server

```rust
extern crate nippy;

#[async_std::main]
async fn main() {
    println!("{:?}", nippy::get_unix_ntp_time().await.unwrap());
}
```
*/
#![recursion_limit = "1024"]

#[macro_use]
extern crate custom_derive;
extern crate conv;
#[macro_use]
extern crate log;
extern crate byteorder;

pub mod protocol;

use protocol::{ReadBytes, ConstPackedSizeBytes, WriteBytes};
use std::io;
use async_std::net::{ToSocketAddrs, UdpSocket};
use anyhow::Result;
use std::{self, time};

/// Send an async request to an ntp server with a hardcoded 5 second timeout.
///
///   `addr` can be any valid socket address
///   returns an error if the server cannot be reached or the response is invalid.
///
pub async fn request<A: ToSocketAddrs>(addr: A) -> io::Result<protocol::Packet> {
    // Create a packet for requesting from an NTP server as a client.
    let mut packet = {
        let leap_indicator = protocol::LeapIndicator::default();
        let version = protocol::Version::V4;
        let mode = protocol::Mode::Client;
        let poll = 0;
        let precision = 0;
        let root_delay = protocol::ShortFormat::default();
        let root_dispersion = protocol::ShortFormat::default();
        let transmit_timestamp = Instant::now().into();
        let stratum = protocol::Stratum::UNSPECIFIED;
        let src = protocol::PrimarySource::Null;
        let reference_id = protocol::ReferenceIdentifier::PrimarySource(src);
        let reference_timestamp = protocol::TimestampFormat::default();
        let receive_timestamp = protocol::TimestampFormat::default();
        let origin_timestamp = protocol::TimestampFormat::default();
        protocol::Packet {
            leap_indicator,
            version,
            mode,
            stratum,
            poll,
            precision,
            root_delay,
            root_dispersion,
            reference_id,
            reference_timestamp,
            origin_timestamp,
            receive_timestamp,
            transmit_timestamp,
        }
    };

    // Write the packet to a slice of bytes.
    let mut bytes = [0u8; protocol::Packet::PACKED_SIZE_BYTES];
    (&mut bytes[..]).write_bytes(&packet)?;

    // Create the socket from which we will send the packet.
    let sock = UdpSocket::bind("0.0.0.0:0").await?;

    // Send the data.
    let sz = sock.send_to(&bytes, addr).await?;
    debug!("{:?}", sock.local_addr());
    debug!("sent: {}", sz);

    // Receive the response.
    let res = sock.recv(&mut bytes[..]).await?;
    debug!("recv: {:?}", res);
    debug!("{:?}", &bytes[..]);

    // Read the received packet from the response.
    packet = (&bytes[..]).read_bytes()?;
    Ok(packet)
}


/// The number of seconds from 1st January 1900 UTC to the start of the Unix epoch.
pub const EPOCH_DELTA: i64 = 2_208_988_800;

// The NTP fractional scale.
const NTP_SCALE: f64 = std::u32::MAX as f64;

/// Describes an instant relative to the `UNIX_EPOCH` - 00:00:00 Coordinated Universal Time (UTC),
/// Thursay, 1 January 1970 in seconds with the fractional part in nanoseconds.
///
/// If the **Instant** describes some moment prior to `UNIX_EPOCH`, both the `secs` and
/// `subsec_nanos` components will be negative.
///
/// The sole purpose of this type is for retrieving the "current" time using the `std::time` module
/// and for converting between the ntp timestamp formats. If you are interested in converting from
/// unix time to some other more human readable format, perhaps see the [chrono
/// crate](https://crates.io/crates/chrono).
///
/// ## Example
///
/// Here is a demonstration of displaying the **Instant** in local time using the chrono crate:
///
/// ```
/// extern crate chrono;
/// extern crate nippy;
///
/// use chrono::TimeZone;
///
/// fn main() {
///     let unix_time = nippy::Instant::now();
///     let local_time = chrono::Local.timestamp(unix_time.secs(), unix_time.subsec_nanos() as _);
///     println!("{}", local_time);
/// }
/// ```
#[derive(Copy, Clone, Debug)]
pub struct Instant {
    secs: i64,
    subsec_nanos: i32,
}

impl Instant {
    /// Create a new **Instant** given its `secs` and `subsec_nanos` components.
    ///
    /// To indicate a time following `UNIX_EPOCH`, both `secs` and `subsec_nanos` must be positive.
    /// To indicate a time prior to `UNIX_EPOCH`, both `secs` and `subsec_nanos` must be negative.
    /// Violating these invariants will result in a **panic!**.
    pub fn new(secs: i64, subsec_nanos: i32) -> Instant {
        if secs > 0 && subsec_nanos < 0 {
            panic!("invalid instant: secs was positive but subsec_nanos was negative");
        }
        if secs < 0 && subsec_nanos > 0 {
            panic!("invalid instant: secs was negative but subsec_nanos was positive");
        }
        Instant { secs, subsec_nanos }
    }

    /// Uses `std::time::SystemTime::now` and `std::time::UNIX_EPOCH` to determine the current
    /// **Instant**.
    ///
    /// ## Example
    ///
    /// ```
    /// extern crate nippy;
    ///
    /// fn main() {
    ///     println!("{:?}", nippy::Instant::now());
    /// }
    /// ```
    pub fn now() -> Self {
        match time::SystemTime::now().duration_since(time::UNIX_EPOCH) {
            Ok(duration) => {
                let secs = duration.as_secs() as i64;
                let subsec_nanos = duration.subsec_nanos() as i32;
                Instant::new(secs, subsec_nanos)
            }
            Err(sys_time_err) => {
                let duration_pre_unix_epoch = sys_time_err.duration();
                let secs = -(duration_pre_unix_epoch.as_secs() as i64);
                let subsec_nanos = -(duration_pre_unix_epoch.subsec_nanos() as i32);
                Instant::new(secs, subsec_nanos)
            }
        }
    }

    /// The "seconds" component of the **Instant**.
    pub fn secs(&self) -> i64 {
        self.secs
    }

    /// The fractional component of the **Instant** in nanoseconds.
    pub fn subsec_nanos(&self) -> i32 {
        self.subsec_nanos
    }
}

// Conversion implementations.

impl From<protocol::ShortFormat> for Instant {
    fn from(t: protocol::ShortFormat) -> Self {
        let secs = t.seconds as i64 - EPOCH_DELTA;
        let subsec_nanos = (t.fraction as f64 / NTP_SCALE * 1e9) as i32;
        Instant::new(secs, subsec_nanos)
    }
}

impl From<protocol::TimestampFormat> for Instant {
    fn from(t: protocol::TimestampFormat) -> Self {
        let secs = t.seconds as i64 - EPOCH_DELTA;
        let subsec_nanos = (t.fraction as f64 / NTP_SCALE * 1e9) as i32;
        Instant::new(secs, subsec_nanos)
    }
}

impl From<Instant> for protocol::ShortFormat {
    fn from(t: Instant) -> Self {
        let sec = t.secs() + EPOCH_DELTA;
        let frac = t.subsec_nanos() as f64 * NTP_SCALE / 1e10;
        protocol::ShortFormat {
            seconds: sec as u16,
            fraction: frac as u16,
        }
    }
}

impl From<Instant> for protocol::TimestampFormat {
    fn from(t: Instant) -> Self {
        let sec = t.secs() + EPOCH_DELTA;
        let frac = t.subsec_nanos() as f64 * NTP_SCALE / 1e10;
        protocol::TimestampFormat {
            seconds: sec as u32,
            fraction: frac as u32,
        }
    }
}

pub async fn get_unix_ntp_time() -> Result<i64> {
    let pool_ntp = "pool.ntp.org:123";
    let response = request(pool_ntp).await?;
    let timestamp = response.transmit_timestamp;
    Ok(Instant::from(timestamp).secs())
}
