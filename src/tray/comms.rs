use crate::comms::async_socket::{AsyncReadObj, AsyncWriteObj};
use crate::comms::{GuiAction, GuiResponse, SOCKET_ADDR};
use crate::tray::GLOBAL_CANCEL;
use crate::until_global_cancel;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_util::sync::CancellationToken;

/// Starts communication between the gui & the tray.
///
/// This method should only be called once, as when a new tray will be connected to when it opens.
pub(crate) async fn init_communication(
    mut sender: Sender<GuiResponse>,
    mut receiver: Receiver<GuiAction>,
) {
    let listener = tokio::net::TcpListener::bind(SOCKET_ADDR)
        .await
        .expect(&format!(
            "Unable to connect listen for gui on {SOCKET_ADDR}"
        ));

    while !GLOBAL_CANCEL.is_cancelled() {
        until_global_cancel!(async {
            let (stream, _) = listener
                .accept()
                .await
                .expect("Unable to accept incoming connection.");

            let (rx, tx) = stream.into_split();
            let close = GLOBAL_CANCEL.child_token();

            tokio::join!(
                read(rx, &mut sender, close.clone()),
                write(tx, &mut receiver, close)
            );
        })
    }
}

/// Reads commuinication from the GUI and sends it internally using a [`Sender`].
async fn read(mut rx: OwnedReadHalf, sender: &mut Sender<GuiResponse>, closed: CancellationToken) {
    closed
        .run_until_cancelled(async {
            let mut run = true;
            while run {
                let response = match rx.read_obj().await {
                    Ok(response) => response,
                    Err(err) => {
                        log::error!("GUI sent invalid data: {err}");
                        closed.cancel();
                        // TODO(tye): if this occurs try close the GUI (somehow).
                        return;
                    }
                };

                run = !matches!(response, GuiResponse::Closed);

                if let Err(_) = sender.send(response).await {
                    log::error!("Failure of internal communication.");
                    GLOBAL_CANCEL.cancel();
                }
            }
            closed.cancel();
        })
        .await;
}

/// Writes data to the GUI from an internal [`Receiver`].
async fn write(
    mut tx: OwnedWriteHalf,
    receiver: &mut Receiver<GuiAction>,
    closed: CancellationToken,
) {
    closed
        .run_until_cancelled(async {
            let mut run = true;
            while run {
                let action = match receiver.recv().await {
                    Some(action) => action,
                    None => {
                        log::error!("Failure of internal communication.");
                        GLOBAL_CANCEL.cancel();
                        return;
                    }
                };

                run = !matches!(action, GuiAction::Close);

                if let Err(err) = tx.write_obj(action).await {
                    log::error!("Unable to send data to GUI: {err}");
                    closed.cancel();
                    return;
                }
            }
        })
        .await;
}
