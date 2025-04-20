use std::{future::Future, num::TryFromIntError};

use crate::{BINCODE_CONF, BincodeConfiguration};
use bincode::{
    Decode, Encode,
    error::{DecodeError, EncodeError},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// An error encountered when trying to read data from an asynchronous source with [`AsyncReadObj`].
#[derive(thiserror::Error, Debug)]
pub enum AsyncReadError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    /// Machine needs to be 64 bit or higher.
    #[error("Cannot convert u64 to usize. Is this a 32-bit machine?")]
    IntConvert(#[from] TryFromIntError),
    /// A more fine variant of [`AsyncReadError::InvalidData`].
    #[error("The number of bytes read does not match the number of bytes requested.")]
    BufferMissMatch { expected: usize, read: usize },
    /// The data received cannot be decoded into the specified type.
    #[error("Data received is not valid.")]
    InvalidData(#[from] DecodeError),
}

/// Reads a data structure from a compatible asynchronous source.
pub trait AsyncReadObj {
    /// Reads the data structure from this source asynchronously.
    ///
    /// # Cancel Safety
    /// This method is **not** cancel safe.
    fn read_obj<Obj>(&mut self) -> impl Future<Output = Result<Obj, AsyncReadError>>
    where
        Obj: Decode<BincodeConfiguration>;
}

impl<From> AsyncReadObj for From
where
    From: AsyncReadExt + Unpin,
{
    fn read_obj<Obj>(&mut self) -> impl Future<Output = Result<Obj, AsyncReadError>>
    where
        Obj: Decode<BincodeConfiguration>,
    {
        async {
            let len: usize = self.read_u64().await?.try_into()?;

            let mut buf: Box<[u8]> = std::iter::from_fn(|| Some(0u8)).take(len).collect();

            let read_exact = self.read_exact(&mut buf).await?;
            if read_exact != len as usize {
                return Err(AsyncReadError::BufferMissMatch {
                    expected: len,
                    read: read_exact,
                });
            }

            Ok(bincode::decode_from_slice_with_context(&buf, BINCODE_CONF, BINCODE_CONF)?.0)
        }
    }
}

/// An error encountered when trying to write data to an asynchronous output with [`AsyncWriteObj`].
#[derive(thiserror::Error, Debug)]
pub enum AsyncWriteError {
    /// Unable to encode the data for writing.
    #[error(transparent)]
    Encode(#[from] EncodeError),
    /// Unable to write data to the output.
    #[error(transparent)]
    Write(#[from] std::io::Error),
}

/// Writes a data structure to a compatible asynchronous output.
pub trait AsyncWriteObj {
    /// Writes the data structure to this output asynchronously.
    ///
    /// # Cancel Safety
    /// This method is **not** cancel safe.
    fn write_obj<Obj>(&mut self, data: Obj) -> impl Future<Output = Result<(), AsyncWriteError>>
    where
        Obj: Encode;
}

impl<To> AsyncWriteObj for To
where
    To: AsyncWriteExt + Unpin,
{
    fn write_obj<Obj>(&mut self, data: Obj) -> impl Future<Output = Result<(), AsyncWriteError>>
    where
        Obj: Encode,
    {
        async {
            let data = bincode::encode_to_vec(data, BINCODE_CONF)?;
            self.write_all(&data.len().to_ne_bytes()).await?;
            self.write_all(data.as_slice()).await?;

            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use interprocess::local_socket::{
        GenericFilePath, ListenerOptions,
        tokio::{Stream, prelude::*},
    };
    use tempfile::TempDir;
    use tokio::task;

    use super::*;

    #[tokio::test]
    async fn end_to_end() {
        let temp_dir = TempDir::new().expect("Able to create temp dir");
        let mut path = temp_dir.path().to_path_buf();
        path.push("sock.sock");

        let name = path
            .to_fs_name::<GenericFilePath>()
            .expect("Unable to start IPC");

        let listener = ListenerOptions::new()
            .name(name.clone())
            .create_tokio()
            .unwrap();

        // Writer
        task::spawn(async move {
            let mut connect = Stream::connect(name).await.unwrap();
            connect.write_obj(GuiAction::Open).await.unwrap();
        });

        // Reader
        task::spawn(async move {
            let mut accept = listener.accept().await.unwrap();
            let read_obj = accept.read_obj::<GuiAction>().await.unwrap();
            assert_eq!(read_obj, GuiAction::Open)
        });
    }
}
