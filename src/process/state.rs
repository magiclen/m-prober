use std::str::FromStr;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ProcessState {
    Running,
    Sleeping,
    Waiting,
    Zombie,
    Stopped,
    TracingStop,
    PagingOrWaking,
    Dead,
    Wakekill,
    Parked,
    Idle,
}

impl ProcessState {
    #[inline]
    pub fn from_str<S: AsRef<str>>(s: S) -> Option<ProcessState> {
        match s.as_ref() {
            "R" => Some(ProcessState::Running),
            "S" => Some(ProcessState::Sleeping),
            "D" => Some(ProcessState::Waiting),
            "Z" => Some(ProcessState::Zombie),
            "T" => Some(ProcessState::Stopped),
            "t" => Some(ProcessState::TracingStop),
            "W" => Some(ProcessState::PagingOrWaking),
            "X" | "x" => Some(ProcessState::Dead),
            "K" => Some(ProcessState::Wakekill),
            "P" => Some(ProcessState::Parked),
            "I" => Some(ProcessState::Idle),
            _ => None,
        }
    }
}

impl ProcessState {
    #[inline]
    pub fn as_str(self) -> &'static str {
        match self {
            ProcessState::Running => "Running",
            ProcessState::Sleeping => "Sleeping",
            ProcessState::Waiting => "Waiting",
            ProcessState::Zombie => "Zombie",
            ProcessState::Stopped => "Stopped",
            ProcessState::TracingStop => "TracingStop",
            ProcessState::PagingOrWaking => "Waking",
            ProcessState::Dead => "Dead",
            ProcessState::Wakekill => "Wakekill",
            ProcessState::Parked => "Parked",
            ProcessState::Idle => "Idle",
        }
    }
}

impl FromStr for ProcessState {
    type Err = ();

    #[inline]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        ProcessState::from_str(s).ok_or(())
    }
}

impl Default for ProcessState {
    #[inline]
    fn default() -> ProcessState {
        ProcessState::Idle
    }
}
