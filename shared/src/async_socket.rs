use std::num::TryFromIntError;

use crate::{BINCODE_CONF, GuiAction};
use bincode::{
    Encode,
    error::{DecodeError, EncodeError},
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(thiserror::Error, Debug)]
enum AsyncReadError {
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error("Cannot convert u64 to usize. Is this a 32-bit machine?")]
    IntConvert(#[from] TryFromIntError),
    #[error("The number of bytes read does not match the number of bytes requested.")]
    BufferMissMatch { expected: usize, read: usize },
    #[error("Data received is not valid.")]
    InvalidData(#[from] DecodeError),
}

trait AsyncReadObj<Obj> {
    async fn read_obj(&mut self) -> Result<Obj, AsyncReadError>;
}

// Didn't implement as generic Obj due to types being annoying.
// Might do it in the future.
impl<To> AsyncReadObj<GuiAction> for To
where
    To: AsyncReadExt + Unpin,
{
    async fn read_obj(&mut self) -> Result<GuiAction, AsyncReadError> {
        let len: usize = self.read_u64().await?.try_into()?;

        let mut buf: Box<[u8]> = std::iter::from_fn(|| Some(0u8)).take(len).collect();

        let read_exact = self.read_exact(&mut buf).await?;
        if read_exact != len as usize {
            return Err(AsyncReadError::BufferMissMatch {
                expected: len,
                read: read_exact,
            });
        }

        Ok(bincode::decode_from_slice_with_context(&buf, BINCODE_CONF, ())?.0)
    }
}

#[derive(thiserror::Error, Debug)]
enum AsyncWriteError {
    #[error(transparent)]
    Encode(#[from] EncodeError),
    #[error(transparent)]
    Write(#[from] std::io::Error),
}

trait AsyncWriteObj<Obj> {
    async fn write_obj(&mut self, data: Obj) -> Result<(), AsyncWriteError>;
}

impl<Obj, To> AsyncWriteObj<Obj> for To
where
    Obj: Encode,
    To: AsyncWriteExt + Unpin,
{
    async fn write_obj(&mut self, data: Obj) -> Result<(), AsyncWriteError> {
        let data = bincode::encode_to_vec(data, BINCODE_CONF)?;
        self.write_all(&data.len().to_ne_bytes()).await?;
        self.write_all(data.as_slice()).await?;

        Ok(())
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
            let read_obj = accept.read_obj().await.unwrap();
            assert_eq!(read_obj, GuiAction::Open)
        });
    }
}
