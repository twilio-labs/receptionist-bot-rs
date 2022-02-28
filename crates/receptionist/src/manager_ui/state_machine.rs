/// scratch pad for possible implementation of state machine for UI flow
/// If implemented, should probably try to use typestate-rs library
/// (as of 12/1/2021, all state machine libs have been investigated and typestate-rs looks most promising for xstate-like usage)

pub enum ResponseEditorStates {
    NoDataToEdit,
    EditingListener,
    EditingConditions,
    EditingActions,
    EditingCollaborators,
    PreSubmitConfirmation,
    Submitted,
}

impl ResponseEditorStates {
    pub fn next_state(&self) -> ResponseEditorStates {
        match self {
            ResponseEditorStates::NoDataToEdit => ResponseEditorStates::EditingListener,
            ResponseEditorStates::EditingListener => ResponseEditorStates::EditingConditions,
            ResponseEditorStates::EditingConditions => ResponseEditorStates::EditingActions,
            ResponseEditorStates::EditingActions => ResponseEditorStates::EditingCollaborators,
            ResponseEditorStates::EditingCollaborators => {
                ResponseEditorStates::PreSubmitConfirmation
            }
            ResponseEditorStates::PreSubmitConfirmation => ResponseEditorStates::Submitted,
            ResponseEditorStates::Submitted => ResponseEditorStates::Submitted,
        }
    }

    pub fn reset(&self) -> ResponseEditorStates {
        return ResponseEditorStates::NoDataToEdit;
    }
}
