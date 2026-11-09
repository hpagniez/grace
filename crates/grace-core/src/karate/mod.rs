//! What Electron Grace's `src/main/karate/` holds: the Karate CLI's side of a run. This part is the run's events:
//! reading `karate-events.jsonl` (`events`), the dry run's tree (`tree`), the run's state (`run_state`), and the steps
//! and HTTP calls of a finished feature (`steps`, `http`).
pub mod events;
pub mod http;
mod js;
pub mod run_state;
pub mod steps;
pub mod tree;
