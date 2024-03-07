//! Utility for testing a Decision implementation
//!
//! The test harness allows you to set up a history of events, perform the given decision,
//! and make assertions about the resulting changes.
use claims::*;
use disintegrate::{Decision, Event, IntoState, IntoStatePart, MultiState, PersistedEvent};
use pretty_assertions::assert_eq;
use std::fmt::Debug;

pub struct TestHarness;

/// Test harness for testing decisions.
impl TestHarness {
    /// Sets up a history of events.
    ///
    /// # Arguments
    ///
    /// * `history` - A history of events to derive the current state.
    ///
    /// # Returns
    ///
    /// A `TestHarnessStep` representing the "given" step.
    pub fn given<E: Event + Clone>(history: impl Into<Vec<E>>) -> TestHarnessStep<E, Given> {
        TestHarnessStep { history: history.into(), _step: Given }
    }
}

/// Represents the given step of the test harness.
pub struct Given;

/// Represents when step of the test harness.
pub struct When<E, ERR> {
    result: Result<Vec<E>, ERR>,
}

pub struct TestHarnessStep<E, ST> {
    history: Vec<E>,
    _step: ST,
}

impl<E: Event + Clone> TestHarnessStep<E, Given> {
    /// Executes a decision on the state derived from the given history.
    ///
    /// # Arguments
    ///
    /// * `decision` - The decision to test.
    ///
    /// # Returns
    ///
    /// A `TestHarnessStep` representing the "when" step.
    pub fn when<D, SP, S, ERR>(self, decision: D) -> TestHarnessStep<E, When<E, ERR>>
    where
        D: Decision<Event = E, Error = ERR, StateQuery = S>,
        S: IntoStatePart<S, Target = SP>,
        SP: IntoState<S> + MultiState<E>,
    {
        let mut state = decision.state_query().into_state_part();
        for event in self
            .history
            .iter()
            .enumerate()
            .map(|(id, event)| PersistedEvent::new((id + 1) as i64, event.clone()))
        {
            state.mutate_all(event);
        }
        let result = decision.process(&state.into_state());
        TestHarnessStep { history: self.history, _step: When { result } }
    }
}

impl<EA, EE, ERR> TestHarnessStep<EE, When<EA, ERR>>
where
    EE: Event + Debug + Clone + PartialEq + PartialEq<EA>,
    EA: Debug + PartialEq,
    ERR: Debug,
{
    /// Makes assertions about the changes.
    ///
    /// # Arguments
    ///
    /// * `expected` - The expected changes.
    ///
    /// # Panics
    ///
    /// Panics if the action result is not `Ok` or if the changes do not match the expected changes.
    ///
    /// # Examples
    #[track_caller]
    pub fn then(self, expected: impl Into<Vec<EE>>) {
        let expected: Vec<_> = expected.into();
        let actual: Vec<_> = assert_ok!(self._step.result).into_iter().map(|e| e).collect();
        assert_eq!(expected, actual);
    }

    /// Makes assertions about the expected error result.
    ///
    /// # Arguments
    ///
    /// * `expected` - The expected error.
    ///
    /// # Panics
    ///
    /// Panics if the action result is not `Err` or if the error does not match the expected error.
    #[track_caller]
    pub fn then_err(self) -> ERR {
        assert_err!(self._step.result)
    }
}
