use serde_json::Value;

/// One Claude Code session, as its hooks report it.
#[derive(Clone, Debug, PartialEq)]
pub struct Session {
    pub(super) id: String,
    /// The last part of its working directory.
    pub(super) project: String,
    pub(super) cwd: String,
    pub(super) model: String,
    pub(super) mode: String,
    pub(super) activity: Activity,
    /// What it is doing now, such as "Editing overview.rs".
    pub(super) doing: String,
    /// Each prompt with what it led to, oldest first.
    pub(super) history: Vec<Exchange>,
    /// Numbers each step, so a step's view keeps its own animation.
    pub(super) seq: u64,
    pub(super) started_at: i64,
    pub(super) active_at: i64,
}

impl Session {
    /// The exchange going on now, or the last one.
    pub(super) fn current(&self) -> Option<&Exchange> {
        self.history.last()
    }

    pub(super) fn reply(&self) -> &str {
        self.current().map_or("", |exchange| exchange.reply.as_str())
    }
}

/// One prompt, the steps it led to, and the reply that ended it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exchange {
    pub(super) prompt: String,
    pub(super) steps: Vec<Step>,
    pub(super) reply: String,
    /// From the session's `seq` as the prompt and the reply came, so what arrives later is told
    /// from what was already there.
    pub(super) seq: u64,
    pub(super) reply_seq: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineKind {
    Same,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiveLine {
    pub(super) number: Option<u32>,
    pub(super) text: String,
    pub(super) kind: LineKind,
}

/// A step as the session view follows it.
#[derive(Clone, Debug, PartialEq)]
pub enum Live {
    /// Lines of a file it read.
    Reading { file: String, lines: Vec<LiveLine> },
    /// A change to a file, as a diff.
    Editing { file: String, lines: Vec<LiveLine> },
    /// A command, and the end of what it printed once it finished.
    Running {
        command: String,
        output: Vec<String>,
        done: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Activity {
    Waiting,
    Working,
    Idle,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepState {
    Done,
    Now,
    Waiting,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub(super) verb: String,
    pub(super) target: String,
    pub(super) state: StepState,
    /// What it read, changed or ran, to follow.
    pub(super) live: Option<Live>,
    /// From the session's `seq`; a new one when its view changes, so it animates afresh.
    pub(super) seq: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Question {
    pub(super) header: String,
    pub(super) text: String,
    pub(super) multi: bool,
    /// Label and description.
    pub(super) options: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    Permission {
        tool: String,
        /// What it would run or touch, such as the command.
        subject: String,
        /// Why, when the tool says.
        reason: String,
        /// The first rule Claude Code suggests, as it sent it, for Always.
        suggestion: Option<Value>,
    },
    Questions {
        questions: Vec<Question>,
        /// The tool's input as sent, echoed back with the answers.
        input: Value,
    },
}

/// Something a session waits on you for.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub(super) id: u64,
    pub(super) session: String,
    pub(super) project: String,
    pub(super) ask: Ask,
    pub(super) received_at: i64,
    pub(super) expires_at: i64,
}

/// Sent to the islands rather than kept in the state.
pub enum Event {
    /// The requests waiting now, by id, so each island shows or drops their pop-ups.
    Requests(Vec<u64>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    Allow,
    Always,
    Deny,
    /// Labels picked for each question, in order.
    Picked(Vec<Vec<String>>),
    /// Leave it to the terminal, as when nobody answers in time.
    Terminal,
}
