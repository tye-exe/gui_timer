use bincode::{
    Decode, Encode,
    error::{DecodeError, EncodeError},
};

use crate::{BINCODE_CONF, BincodeConfiguration};

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
pub trait ReadObj {
    /// Reads the data structure from this source.
    fn read_obj<Obj>(&mut self) -> Result<Obj, ReadError>
    where
        Obj: Decode<BincodeConfiguration>;
}

impl<From: std::io::Read> ReadObj for From {
    fn read_obj<Obj>(&mut self) -> Result<Obj, ReadError>
    where
        Obj: Decode<BincodeConfiguration>,
    {
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
pub trait WriteObj {
    /// Writes the given data to this output.
    fn write_obj<Obj: Encode>(&mut self, data: Obj) -> Result<(), WriteError>;
}

impl<To: std::io::Write> WriteObj for To {
    fn write_obj<Obj: Encode>(&mut self, data: Obj) -> Result<(), WriteError> {
        let data = bincode::encode_to_vec(data, BINCODE_CONF)?;
        self.write_all(&data.len().to_ne_bytes())?;
        self.write_all(data.as_slice())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use interprocess::local_socket::{
        GenericFilePath, ListenerOptions, Stream, ToFsName as _,
        traits::{Listener, Stream as _},
    };
    use tempfile::TempDir;

    use crate::{
        GuiAction,
        sync_socket::{ReadObj as _, WriteObj as _},
    };

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
            .write_obj(GuiAction::Close)
            .expect("Can write");

        let mut connect = listener.accept().unwrap();

        let gui_action = connect.read_obj::<GuiAction>().expect("Can read");

        assert_eq!(gui_action, GuiAction::Close);
    }
}
