use strum_macros::EnumString;

pub mod twitter;

#[derive(EnumString)]
pub enum LiveState {
    Running,
    Ended
}
