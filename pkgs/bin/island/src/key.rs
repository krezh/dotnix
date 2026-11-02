pub struct Key {
    pub name: String,
    /// The text it types, if any.
    pub typed: Option<String>,
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
}
