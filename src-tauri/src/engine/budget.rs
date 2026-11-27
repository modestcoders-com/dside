//! The round budget of one turn, and the checkpoint that comes with running out of it.
//!
//! A turn is not one model call. The model asks, a tool answers, the model asks again — every
//! request is one round, and a large task needs many. The ceiling exists so a model that loops
//! cannot keep a turn alive forever, but a hard stop in the middle of a step is the worst way to
//! enforce it: the work is half done, the user sees a token like `max_turns`, and nothing says
//! what happened.
//!
//! So the ceiling is a checkpoint, not a wall. When the first budget is used up the model is
//! asked to account for the rounds — with facts the engine counted, not with a question it can
//! answer out of habit — and then gets one more budget. When that is used up too, it gets a last
//! round without tools, so the turn ends with a summary instead of a cut.
//!
//! **Who does the checking.** Not a second model and not a parallel watcher: the numbers that
//! expose a runaway are mechanical. How many rounds of which tool, whether the same command ran
//! again and again, how many files were touched. A model asked "are you on the right path?" says
//! yes; a model told "you ran the same build eleven times" has to deal with that. The facts come
//! from here; the judgement stays the model's, and the user reads both.

use std::collections::BTreeMap;

use serde_json::Value;

/// Rounds of work before the first checkpoint. Matches the engine default; the turn's configured
/// value wins, this is only the shape of the budget.
pub const WORK_ROUNDS: u32 = 24;
/// What one checkpoint is worth. One extension, not a series: a model that needs two has a
/// different problem, and the second summary is already late.
pub const EXTENSION_ROUNDS: u32 = 24;
/// How many rounds before the end the model is told how few are left.
const WARN_WITHIN: u32 = 3;

/// Counts what one turn did, so the checkpoint can state it instead of asking about it.
#[derive(Default)]
pub struct Ledger {
    rounds: u32,
    budget: u32,
    extended: bool,
    wrapped: bool,
    tools: BTreeMap<String, u32>,
    /// Normalised command lines, so `cargo test` and `cargo  test ` count as one.
    commands: BTreeMap<String, u32>,
    files_written: u32,
}

impl Ledger {
    pub fn new(work_rounds: u32) -> Self {
        Self {
            budget: work_rounds.max(1),
            ..Self::default()
        }
    }

    pub fn rounds(&self) -> u32 {
        self.rounds
    }

    pub fn budget(&self) -> u32 {
        self.budget
    }

    pub fn is_wrapped(&self) -> bool {
        self.wrapped
    }

    /// Records one round of the model's tool calls.
    pub fn record(&mut self, calls: &[Value]) {
        self.rounds += 1;
        for call in calls {
            let name = call
                .pointer("/function/name")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string();
            *self.tools.entry(name.clone()).or_insert(0) += 1;
            if name == "write" || name == "edit" {
                self.files_written += 1;
            }
            if name == "bash" {
                let raw = call
                    .pointer("/function/arguments")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if let Some(command) = bash_command(raw) {
                    let key = normalise(&command);
                    *self.commands.entry(key).or_insert(0) += 1;
                }
            }
        }
    }

    /// True while a round has to be refused. Otherwise the caller runs the round.
    ///
    /// This is where the ceiling becomes a checkpoint: the first time it has to refuse, the
    /// budget grows and the review is planted; the second time it grants the single wrap-up
    /// round that ends the turn with words.
    pub fn may_run(&mut self) -> Verdict {
        if self.rounds < self.budget {
            return Verdict::Run;
        }
        if !self.extended {
            self.extended = true;
            self.budget += EXTENSION_ROUNDS;
            return Verdict::Review;
        }
        if !self.wrapped {
            self.wrapped = true;
            self.budget += 1;
            return Verdict::WrapUp;
        }
        Verdict::Stop
    }

    /// Rounds left after this one, as the model should be told — only near the end, because a
    /// reminder in every round is noise that trains the model to ignore it.
    pub fn note(&self) -> Option<String> {
        let left = self.budget.saturating_sub(self.rounds);
        if left == 0 || left > WARN_WITHIN {
            return None;
        }
        Some(format!(
            "Rounds: {self_spent} of {budget} used in this turn — {left} left. Finish the step you \
             are in and report what is done; if more is open, say what it is.",
            self_spent = self.rounds,
            budget = self.budget,
            left = left
        ))
    }

    /// The facts, as one line the model cannot argue with.
    pub fn facts(&self) -> String {
        let mut parts = Vec::new();
        if !self.tools.is_empty() {
            let counts: Vec<String> = self
                .tools
                .iter()
                .map(|(name, count)| format!("{name} {count}×"))
                .collect();
            parts.push(format!("tool calls: {}", counts.join(", ")));
        }
        if let Some((command, count)) = self.most_repeated_command() {
            parts.push(format!(
                "the same command ran {count}×: `{}`",
                shorten(&command)
            ));
        }
        if self.files_written > 0 {
            parts.push(format!("{} file write(s) or edit(s)", self.files_written));
        }
        if parts.is_empty() {
            return "no tool calls happened in this turn".to_string();
        }
        parts.join("; ")
    }

    fn most_repeated_command(&self) -> Option<(&String, u32)> {
        self.commands
            .iter()
            .max_by_key(|(_, count)| **count)
            .filter(|(_, count)| **count >= 2)
            .map(|(command, count)| (command, *count))
    }

    /// The checkpoint. Facts first, then the three questions that cost a runaway turn its
    /// momentum — and the instruction to answer them, not to restate the plan.
    pub fn review(&self) -> String {
        format!(
            "Checkpoint: this turn has used its {budget} rounds, and the work is not finished. \
             Before you continue, answer these in five lines at most — a real answer, not a plan:\n\
             1. What is finished, and which check says so?\n\
             2. What is still open?\n\
             3. Are you repeating yourself, and is this much content really needed — or is a \
             smaller change the right one?\n\
             \n\
             What this turn actually did: {facts}.\n\
             \n\
             You get {more} more rounds either way. If the answer above shows the approach is \
             wrong, change it now instead of spending the second budget on the same mistake. If \
             the work is done, say so and end the turn.",
            budget = self.budget - EXTENSION_ROUNDS,
            facts = self.facts(),
            more = EXTENSION_ROUNDS
        )
    }

    /// The last round: no tools, so the turn ends with the model's own words instead of a
    /// truncated step and a token in red.
    pub fn wrap_up(&self) -> String {
        format!(
            "The rounds of this turn are used up, and no tool is available for this last answer. \
             Write the hand-over: what is done and verified, what is still open, and — in one \
             sentence — what you would do next. Do not start new work.\n\
             \n\
             What this turn did: {facts}.",
            facts = self.facts()
        )
    }
}

/// What the caller does with a budget that has to decide.
#[derive(Debug, PartialEq, Eq)]
pub enum Verdict {
    /// A normal round.
    Run,
    /// The first budget is spent: plant the review, then run.
    Review,
    /// The extension is spent too: one last round without tools.
    WrapUp,
    /// Nothing left to grant.
    Stop,
}

/// The command out of a `bash` call's JSON arguments.
fn bash_command(arguments: &str) -> Option<String> {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| {
            value
                .get("command")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn normalise(command: &str) -> String {
    command.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn shorten(command: &str) -> String {
    let cut: String = command.chars().take(90).collect();
    if command.chars().count() > 90 {
        format!("{cut}…")
    } else {
        cut
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn bash(command: &str) -> Value {
        json!({
            "id": "c",
            "type": "function",
            "function": { "name": "bash", "arguments": json!({ "command": command }).to_string() }
        })
    }

    fn read(path: &str) -> Value {
        json!({
            "id": "c",
            "type": "function",
            "function": { "name": "read", "arguments": json!({ "path": path }).to_string() }
        })
    }

    #[test]
    fn the_budget_runs_then_reviews_then_wraps_then_stops() {
        let mut ledger = Ledger::new(3);
        for _ in 0..3 {
            assert_eq!(ledger.may_run(), Verdict::Run);
            ledger.record(&[read("a.txt")]);
        }
        // The fourth round is refused once, and that refusal is the checkpoint.
        assert_eq!(ledger.may_run(), Verdict::Review);
        assert_eq!(ledger.budget(), 3 + EXTENSION_ROUNDS);
        for _ in 0..EXTENSION_ROUNDS {
            assert_eq!(ledger.may_run(), Verdict::Run);
            ledger.record(&[read("a.txt")]);
        }
        assert_eq!(ledger.may_run(), Verdict::WrapUp);
        assert!(ledger.is_wrapped());
        assert_eq!(ledger.may_run(), Verdict::Run);
        ledger.record(&[read("a.txt")]);
        assert_eq!(ledger.may_run(), Verdict::Stop);
    }

    #[test]
    fn a_turn_that_finishes_early_never_sees_the_checkpoint() {
        let mut ledger = Ledger::new(24);
        ledger.record(&[read("a.txt")]);
        assert_eq!(ledger.may_run(), Verdict::Run);
        assert_eq!(ledger.budget(), 24);
        assert!(!ledger.is_wrapped());
    }

    #[test]
    fn the_warning_appears_only_in_the_last_three_rounds() {
        let mut ledger = Ledger::new(10);
        for _ in 0..6 {
            ledger.may_run();
            ledger.record(&[read("a.txt")]);
        }
        assert_eq!(ledger.note(), None, "four rounds left is not worth a reminder");
        for _ in 0..3 {
            ledger.may_run();
            ledger.record(&[read("a.txt")]);
        }
        assert_eq!(ledger.rounds(), 9);
        let note = ledger.note().expect("three left is");
        assert!(note.contains("1 left"), "{note}");
        assert!(note.contains("9 of 10"), "{note}");
    }

    #[test]
    fn the_facts_name_the_tools_and_a_repeated_command() {
        let mut ledger = Ledger::new(24);
        ledger.record(&[
            bash("cargo test --lib"),
            bash("cargo  test   --lib"),
            bash("git status"),
        ]);
        let facts = ledger.facts();
        assert!(facts.contains("bash 3×"), "{facts}");
        assert!(facts.contains("the same command ran 2×"), "{facts}");
        assert!(facts.contains("cargo test --lib"), "{facts}");
    }

    #[test]
    fn writes_and_edits_are_counted_as_touched_files() {
        let mut ledger = Ledger::new(24);
        let edit = json!({
            "id": "c", "type": "function",
            "function": { "name": "edit", "arguments": "{\"path\":\"a.rs\"}" }
        });
        let write = json!({
            "id": "c", "type": "function",
            "function": { "name": "write", "arguments": "{\"path\":\"b.rs\"}" }
        });
        ledger.record(&[edit, write]);
        assert!(ledger.facts().contains("2 file write(s) or edit(s)"));
    }

    #[test]
    fn a_turn_without_tools_says_so_instead_of_showing_an_empty_line() {
        let ledger = Ledger::new(24);
        assert_eq!(ledger.facts(), "no tool calls happened in this turn");
    }

    #[test]
    fn the_review_asks_for_evidence_and_mentions_the_second_budget() {
        let mut ledger = Ledger::new(24);
        for _ in 0..24 {
            ledger.may_run();
            ledger.record(&[bash("cargo build")]);
        }
        let review = ledger.review();
        assert!(review.contains("which check says so"), "{review}");
        assert!(review.contains("24 more rounds"), "{review}");
        assert!(review.contains("cargo build"), "{review}");
    }

    #[test]
    fn the_wrap_up_forbids_new_work() {
        let wrap = Ledger::new(24).wrap_up();
        assert!(wrap.contains("Do not start new work"), "{wrap}");
    }

    #[test]
    fn a_broken_arguments_blob_does_not_panic() {
        let mut ledger = Ledger::new(4);
        let call = json!({
            "id": "c", "type": "function",
            "function": { "name": "bash", "arguments": "not json" }
        });
        ledger.record(&[call]);
        assert!(ledger.facts().contains("bash 1×"));
        assert!(!ledger.facts().contains("the same command"));
    }
}
