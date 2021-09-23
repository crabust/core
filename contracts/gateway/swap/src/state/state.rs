use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{StdResult, Storage};
use cosmwasm_storage::{ReadonlySingleton, Singleton};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub static KEY_STATE: &[u8] = b"state";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub total_supply: Uint256,
}

pub fn store<S: Storage>(storage: &mut S, data: &State) -> StdResult<()> {
    Singleton::new(storage, KEY_STATE).save(data)
}

pub fn read<S: Storage>(storage: &S) -> StdResult<State> {
    ReadonlySingleton::new(storage, KEY_STATE).load()
}
