use anyhow::{Result, anyhow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmState {
    Stopped,
    Starting,
    Running { pid: Option<i32> },
    Stopping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmStateEvent {
    StartCommandIssued,
    ProcessObserved { pid: Option<i32> },
    GuestShutdownObserved,
    StopCommandIssued,
    ForceKillIssued,
    ProcessExited,
}

impl VmState {
    pub fn transition(self, event: VmStateEvent) -> Result<Self> {
        match (self, event) {
            (VmState::Stopped, VmStateEvent::StartCommandIssued) => Ok(VmState::Starting),
            (VmState::Starting, VmStateEvent::ProcessObserved { pid }) => {
                Ok(VmState::Running { pid })
            }
            (VmState::Running { .. }, VmStateEvent::ProcessObserved { pid }) => {
                Ok(VmState::Running { pid })
            }
            (VmState::Running { .. }, VmStateEvent::GuestShutdownObserved) => Ok(VmState::Stopping),
            (VmState::Running { .. }, VmStateEvent::StopCommandIssued)
            | (VmState::Running { .. }, VmStateEvent::ForceKillIssued) => Ok(VmState::Stopping),
            (VmState::Stopping, VmStateEvent::GuestShutdownObserved) => Ok(VmState::Stopping),
            (VmState::Stopping, VmStateEvent::ProcessExited)
            | (VmState::Running { .. }, VmStateEvent::ProcessExited)
            | (VmState::Starting, VmStateEvent::ProcessExited) => Ok(VmState::Stopped),
            (state, event) => Err(anyhow!(
                "invalid VM state transition: state={:?}, event={:?}",
                state,
                event
            )),
        }
    }

    pub fn status_label(&self) -> &'static str {
        match self {
            VmState::Stopped => "Not running",
            VmState::Starting => "Starting",
            VmState::Running { .. } => "Running",
            VmState::Stopping => "Stopping",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{VmState, VmStateEvent};

    #[test]
    fn running_lifecycle_transition_flow_is_valid() {
        let state = VmState::Stopped
            .transition(VmStateEvent::StartCommandIssued)
            .expect("start transition should succeed")
            .transition(VmStateEvent::ProcessObserved { pid: Some(1234) })
            .expect("running transition should succeed")
            .transition(VmStateEvent::StopCommandIssued)
            .expect("stop transition should succeed")
            .transition(VmStateEvent::ProcessExited)
            .expect("exit transition should succeed");

        assert_eq!(state, VmState::Stopped);
    }

    #[test]
    fn invalid_transition_is_rejected() {
        let err = VmState::Stopped
            .transition(VmStateEvent::StopCommandIssued)
            .expect_err("invalid transition should fail");

        assert!(err.to_string().contains("invalid VM state transition"));
    }

    #[test]
    fn force_kill_path_transitions_back_to_stopped() {
        let state = VmState::Running { pid: Some(42) }
            .transition(VmStateEvent::ForceKillIssued)
            .expect("force kill transition should succeed")
            .transition(VmStateEvent::ProcessExited)
            .expect("exit transition should succeed");

        assert_eq!(state, VmState::Stopped);
    }

    #[test]
    fn process_observed_without_start_signal_is_rejected() {
        let err = VmState::Stopped
            .transition(VmStateEvent::ProcessObserved { pid: Some(7) })
            .expect_err("observed process from stopped state should fail");

        assert!(err.to_string().contains("invalid VM state transition"));
    }

    #[test]
    fn running_state_updates_observed_pid() {
        let state = VmState::Running { pid: Some(1) }
            .transition(VmStateEvent::ProcessObserved { pid: Some(2) })
            .expect("pid refresh should succeed");

        assert_eq!(state, VmState::Running { pid: Some(2) });
    }

    #[test]
    fn guest_shutdown_observation_transitions_running_to_stopping() {
        let state = VmState::Running { pid: Some(1) }
            .transition(VmStateEvent::GuestShutdownObserved)
            .expect("guest shutdown observation should succeed");

        assert_eq!(state, VmState::Stopping);
    }
}
