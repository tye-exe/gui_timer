use bincode::{
    Decode, Encode,
    config::{self, Configuration},
};

pub mod async_socket;
pub mod sync_socket;

/// Actions to be perfomced by the timer GUI.
#[derive(Decode, Encode)]
#[cfg_attr(test, derive(PartialEq, Debug))]
pub enum GuiAction {
    Close,
    Open,
}

/// A type alias for the bincode configuration used in this codebase.
type BincodeConfiguration = Configuration<config::BigEndian, config::Fixint>;

/// Default [bincode] configuration used to encode & decode data.
const BINCODE_CONF: BincodeConfiguration = config::standard()
    .with_big_endian()
    .with_fixed_int_encoding();
