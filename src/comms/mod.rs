use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bincode::{
    Decode, Encode,
    config::{self, Configuration},
};

pub mod async_socket;
pub mod sync_socket;

pub const SOCKET_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 23408);

/// Actions to be performed by the timer GUI.
#[derive(Decode, Encode, PartialEq, Debug)]
pub enum GuiAction {
    /// Close the GUI and send confirmation to the tray.
    Close,
    /// Close the GUI **without** sending confirmation to the tray.
    Quit,
}

/// Actions that have been performed by the timer GUI.
#[derive(Decode, Encode, PartialEq, Debug)]
pub enum GuiResponse {
    Opened,
    Closed,
}

/// A type alias for the bincode configuration used in this codebase.
type BincodeConfiguration = Configuration<config::BigEndian, config::Fixint>;

/// Default [bincode] configuration used to encode & decode data.
const BINCODE_CONF: BincodeConfiguration = config::standard()
    .with_big_endian()
    .with_fixed_int_encoding();
