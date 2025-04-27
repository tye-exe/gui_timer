use crate::comms::async_socket::{AsyncReadObj, AsyncWriteObj};
use crate::comms::{GuiAction, GuiResponse, TO_GUI_SOCK, TO_TRAY_SOCK};
use crate::tray::GLOBAL_CANCEL;
use crate::until_global_cancel;
use interprocess::local_socket::tokio::Stream;
use interprocess::local_socket::traits::tokio::Listener;
use interprocess::local_socket::{
    GenericFilePath, GenericNamespaced, ListenerOptions, NameType, ToFsName, ToNsName,
};
use tokio::sync::mpsc::{self, Receiver, Sender, UnboundedReceiver};

/// Sends [`GuiAction`] to the GUI.
async fn gui_send(mut receiver: Receiver<GuiAction>, mut cancel_inform: UnboundedReceiver<()>) {
    let listener = create_listener(TO_GUI_SOCK);
    loop {
        let mut stream = until_global_cancel!(listener.accept())
            .expect("Unable to listen for GUI communication.");

        tokio::select! {
            response = cancel_inform.recv() => {
                if let None = response {
                    log::error!("Unable to communicate with GUI receive task.");
                    GLOBAL_CANCEL.cancel();
                }
            }
            _ = to_gui(&mut stream, &mut receiver) => {}
        };
    }
}

/// Split into separate function due to formatting.
async fn to_gui(stream: &mut Stream, receiver: &mut Receiver<GuiAction>) {
    while let Some(action) = until_global_cancel!(receiver.recv()) {
        let stop = matches!(action, GuiAction::Close);

        log::debug!("Sent Action: {action:?}");

        let _ = stream
            .write_obj::<GuiAction>(action)
            .await
            .inspect_err(|e| log::error!("The GUI was closed unexpectedly? {e}"));

        if stop {
            break;
        }
    }
}

/// Starts communication with the GUI.
///
/// This method should only be called once, as communication will be reestablished when a new GUI is opened.
pub(crate) fn gui_communication(sender: Sender<GuiResponse>, receiver: Receiver<GuiAction>) {
    let (cancel_tx, cancel_rx) = mpsc::unbounded_channel();

    tokio::spawn(gui_send(receiver, cancel_rx));
    tokio::spawn(gui_receive(sender, cancel_tx));
}

/// Receives [`GuiResponse`]s from the GUI and forwards them to receivers in the tray.
async fn gui_receive(sender: Sender<GuiResponse>, cancel_tx: mpsc::UnboundedSender<()>) {
    let listener = create_listener(TO_TRAY_SOCK);
    loop {
        let mut stream = until_global_cancel!(listener.accept())
            .expect("Unable to listen for gui communication.");

        loop {
            let response = match until_global_cancel!(stream.read_obj()) {
                Ok(response) => response,
                Err(err) => {
                    log::error!("Error reading GUI response: {err}");
                    break;
                }
            };

            let stop = matches!(response, GuiResponse::Closed);

            log::debug!("Received Response: {response:?}");
            log::log!(log::Level::Trace, "Is closed: {stop}");

            until_global_cancel!(sender.send(response)).expect("Unable to listen for GUI response");

            if stop {
                if let Err(_) = cancel_tx.send(()) {
                    log::error!("Unable to communicate with gui send task.");
                    GLOBAL_CANCEL.cancel();
                }
                break;
            }
        }
    }
}

/// Creates a listener that listens for connections from the GUI.
fn create_listener(name: &'static str) -> interprocess::local_socket::tokio::Listener {
    let name = match GenericNamespaced::is_supported() {
        true => name.to_ns_name::<GenericNamespaced>(),
        false => format!("/tmp/{}", name).to_fs_name::<GenericFilePath>(),
    }
    .expect("Unable to start IPC");

    ListenerOptions::new()
        .name(name.clone())
        .create_tokio()
        .expect("Unable to listen for gui communication.")
}
