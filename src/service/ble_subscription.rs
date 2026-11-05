use iced::futures::SinkExt;
use iced::stream;
use iced::Subscription;
use tokio::sync::mpsc;

use crate::service::ble_protocol::{BleCommand, BleEvent};
use crate::service::ble_session::{self, SessionControl, SessionState};

#[derive(Debug, Clone)]
pub enum BleSubscriptionEvent {
    Ready {
        command_tx: mpsc::Sender<BleCommand>,
        control_tx: mpsc::Sender<SessionControl>,
    },
    StateChanged(SessionState),
    Event(BleEvent),
    Disconnected(String),
}

pub fn ble_subscription() -> Subscription<BleSubscriptionEvent> {
    Subscription::run(|| {
        stream::channel(64, async move |mut output| {
            let handle = ble_session::start_session();

            let _ = output
                .send(BleSubscriptionEvent::Ready {
                    command_tx: handle.command_tx,
                    control_tx: handle.control_tx,
                })
                .await;

            let mut event_rx = handle.event_rx;
            let mut state_rx = handle.state_rx;

            loop {
                tokio::select! {
                    Ok(()) = state_rx.changed() => {
                        let state = *state_rx.borrow();
                        let _ = output.send(BleSubscriptionEvent::StateChanged(state)).await;
                    }
                    Some(event) = event_rx.recv() => {
                        match &event {
                            BleEvent::Disconnected(reason) => {
                                let _ = output
                                    .send(BleSubscriptionEvent::Disconnected(reason.clone()))
                                    .await;
                            }
                            _ => {
                                let _ = output.send(BleSubscriptionEvent::Event(event)).await;
                            }
                        }
                    }
                    else => break,
                }
            }
        })
    })
}
