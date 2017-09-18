use trait_handle::TraitHandle;

pub type StepTarget = extern "C" fn (result: &mut StepResult, call_context: usize);

#[derive(Default)]
pub struct Chain {
    pub steps: Vec<Step>
}

#[repr(C)]
pub struct Step {
    pub target: StepTarget,
    pub call_context: usize
}

#[repr(C)]
pub struct StepResult {
    pub valid: i32,
    pub last_value: usize,
    pub current_value: usize
}

pub trait ChainExecutor {
}

impl Chain {
    pub fn new() -> Chain {
        Chain::default()
    }
}
