/// A key held with others, which a panel taking the keyboard is told of as it comes up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modifier {
    Control,
    Shift,
    Alt,
    Super,
}

pub struct Key {
    pub name: String,
    /// The text it types, if any.
    pub typed: Option<String>,
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
}
