use tokio::sync::mpsc;
use tracing::{debug, info};

use crate::message::Message;

/// Default channel buffer size for inter-component messaging.
const DEFAULT_BUFFER_SIZE: usize = 256;

/// The central message bus connecting all ClawForge components.
///
/// Each component gets a `Sender` to push messages and a `Receiver` to consume them.
/// Built on Tokio mpsc channels for async, bounded backpressure.
pub struct ClawBus {
    pub scheduler_tx: mpsc::Sender<Message>,
    pub scheduler_rx: Option<mpsc::Receiver<Message>>,

    pub planner_tx: mpsc::Sender<Message>,
    pub planner_rx: Option<mpsc::Receiver<Message>>,

    pub executor_tx: mpsc::Sender<Message>,
    pub executor_rx: Option<mpsc::Receiver<Message>>,

    pub supervisor_tx: mpsc::Sender<Message>,
    pub supervisor_rx: Option<mpsc::Receiver<Message>>,
}

impl ClawBus {
    /// Create a new bus with default buffer sizes.
    pub fn new() -> Self {
        Self::with_buffer_size(DEFAULT_BUFFER_SIZE)
    }

    /// Create a new bus with a custom buffer size.
    pub fn with_buffer_size(buffer: usize) -> Self {
        let (scheduler_tx, scheduler_rx) = mpsc::channel(buffer);
        let (planner_tx, planner_rx) = mpsc::channel(buffer);
        let (executor_tx, executor_rx) = mpsc::channel(buffer);
        let (supervisor_tx, supervisor_rx) = mpsc::channel(buffer);

        info!(buffer_size = buffer, "ClawBus initialized");

        Self {
            scheduler_tx,
            scheduler_rx: Some(scheduler_rx),
            planner_tx,
            planner_rx: Some(planner_rx),
            executor_tx,
            executor_rx: Some(executor_rx),
            supervisor_tx,
            supervisor_rx: Some(supervisor_rx),
        }
    }

    /// Take the scheduler receiver (can only be called once).
    pub fn take_scheduler_rx(&mut self) -> Option<mpsc::Receiver<Message>> {
        debug!("Scheduler receiver taken");
        self.scheduler_rx.take()
    }

    /// Take the planner receiver (can only be called once).
    pub fn take_planner_rx(&mut self) -> Option<mpsc::Receiver<Message>> {
        debug!("Planner receiver taken");
        self.planner_rx.take()
    }

    /// Take the executor receiver (can only be called once).
    pub fn take_executor_rx(&mut self) -> Option<mpsc::Receiver<Message>> {
        debug!("Executor receiver taken");
        self.executor_rx.take()
    }

    /// Take the supervisor receiver (can only be called once).
    pub fn take_supervisor_rx(&mut self) -> Option<mpsc::Receiver<Message>> {
        debug!("Supervisor receiver taken");
        self.supervisor_rx.take()
    }
}

impl Default for ClawBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{JobTrigger, Message};
    use uuid::Uuid;

    #[tokio::test]
    async fn test_bus_send_receive() {
        let mut bus = ClawBus::new();
        let mut rx = bus.take_scheduler_rx().unwrap();

        let run_id = Uuid::new_v4();
        let msg = Message::ScheduleJob(JobTrigger {
            run_id,
            agent_id: Uuid::new_v4(),
            trigger_reason: "test".into(),
        });

        bus.scheduler_tx.send(msg).await.unwrap();
        let received = rx.recv().await.unwrap();
        assert_eq!(received.run_id(), run_id);
    }

    #[tokio::test]
    async fn test_bus_take_rx_once() {
        let mut bus = ClawBus::new();
        assert!(bus.take_planner_rx().is_some());
        assert!(bus.take_planner_rx().is_none()); // second take is None
    }

    #[tokio::test]
    async fn test_bus_backpressure() {
        let mut bus = ClawBus::with_buffer_size(2);
        let _rx = bus.take_executor_rx().unwrap();

        // Fill the buffer
        for _ in 0..2 {
            bus.executor_tx
                .send(Message::ScheduleJob(JobTrigger {
                    run_id: Uuid::new_v4(),
                    agent_id: Uuid::new_v4(),
                    trigger_reason: "fill".into(),
                }))
                .await
                .unwrap();
        }

        // Third send should not complete immediately (buffer full)
        let result = bus.executor_tx.try_send(Message::ScheduleJob(JobTrigger {
            run_id: Uuid::new_v4(),
            agent_id: Uuid::new_v4(),
            trigger_reason: "overflow".into(),
        }));
        assert!(result.is_err());
    }
}
