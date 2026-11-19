#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Discover,
    Setup,
    StandBy,
    Server,
    Finish,
}
