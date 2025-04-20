use std::io::{Read, Write};

use bincode::error::{DecodeError, EncodeError};
use interprocess::local_socket::Stream;

use crate::{BINCODE_CONF, GuiAction};

/// An error encountered when reading a data structure with [`ReadObj`].
#[derive(thiserror::Error, Debug)]
pub enum ReadError {
    /// Unable to read from the data source.
    #[error("Unable to read data.")]
    Read(#[from] std::io::Error),
    /// Unable to decode data read from the source.
    #[error("Unable to encode data.")]
    Decode(#[from] DecodeError),
}

/// For reading data structures from a comptable source.
pub trait ReadObj<T> {
    /// Reads the data structure from this source.
    fn read_obj(&mut self) -> Result<T, ReadError>;
}

impl ReadObj<GuiAction> for Stream {
    fn read_obj(&mut self) -> Result<GuiAction, ReadError> {
        self.read_exact(&mut [0; (usize::BITS / 8) as usize])?;

        Ok(bincode::decode_from_std_read_with_context(
            self,
            BINCODE_CONF,
            BINCODE_CONF,
        )?)
    }
}

/// An error encounctered when writing a data structure with [`WriteObj`].
#[derive(thiserror::Error, Debug)]
pub enum WriteError {
    /// Unable to encoded data for writing.
    #[error("Unable to encode data.")]
    Encode(#[from] EncodeError),
    /// Unable to write data to the output.
    #[error("Unable to write data.")]
    Write(#[from] std::io::Error),
}

/// For writing data structure to a compatible output.
pub trait WriteObj<T> {
    /// Writes the given data to this output.
    fn write_obj(&mut self, data: T) -> Result<(), WriteError>;
}

impl WriteObj<GuiAction> for Stream {
    fn write_obj(&mut self, data: GuiAction) -> Result<(), WriteError> {
        let data = bincode::encode_to_vec(data, BINCODE_CONF)?;
        self.write_all(&data.len().to_ne_bytes())?;
        self.write_all(data.as_slice())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use interprocess::local_socket::{
        GenericFilePath, ListenerOptions, ToFsName as _,
        traits::{Listener, Stream as _},
    };
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn end_to_end() {
        let temp_dir = TempDir::new().expect("Able to create temp dir");
        let mut path = temp_dir.path().to_path_buf();
        path.push("sock.sock");

        let name = path
            .to_fs_name::<GenericFilePath>()
            .expect("Unable to start IPC");

        let opts = ListenerOptions::new().name(name.clone());
        let listener = opts.create_sync().unwrap();

        Stream::connect(name.clone())
            .unwrap()
            .write_obj(GuiAction::Open)
            .expect("Can write");

        let mut connect = listener.accept().unwrap();

        let gui_action = connect.read_obj().expect("Can read");

        assert_eq!(gui_action, GuiAction::Open);
    }
}
