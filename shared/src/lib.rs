use std::{
    io::{Read, Write},
    num::TryFromIntError,
};

use ::tokio::io::AsyncReadExt;
use bincode::{
    Decode, Encode,
    config::{self, Configuration},
    error::{DecodeError, EncodeError},
};
use interprocess::{
    bound_util::RefTokioAsyncRead,
    local_socket::{Stream, tokio::Stream as tokio_stream},
};
use serde::{Deserialize, Serialize};

const BINCODE_CONF: Configuration<config::BigEndian, config::Fixint> = config::standard()
    .with_big_endian()
    .with_fixed_int_encoding();

#[derive(Serialize, Deserialize, Decode, Encode)]
#[cfg_attr(test, derive(PartialEq, Debug))]
enum GuiAction {
    Close,
    Open,
}

trait ReadObj<T> {
    fn read_obj(&mut self) -> Result<T, std::io::Error>;
}

impl ReadObj<GuiAction> for Stream {
    fn read_obj(&mut self) -> Result<GuiAction, std::io::Error> {
        self.read_exact(&mut [0; (usize::BITS / 8) as usize])?;

        Ok(bincode::decode_from_std_read_with_context(self, BINCODE_CONF, BINCODE_CONF).unwrap())
    }
}

#[derive(thiserror::Error, Debug)]
enum WriteErr {
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Write(#[from] std::io::Error),
}

trait WriteObj<T> {
    fn write_obj(&mut self, data: T) -> Result<(), WriteErr>;
}

impl WriteObj<GuiAction> for Stream {
    fn write_obj(&mut self, data: GuiAction) -> Result<(), WriteErr> {
        let data = bincode::encode_to_vec(data, BINCODE_CONF)?;
        self.write(&data.len().to_ne_bytes())?;
        self.write(data.as_slice())?;

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
