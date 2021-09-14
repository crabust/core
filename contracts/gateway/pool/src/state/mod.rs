use cosmwasm_std::StdResult;

pub mod config;
pub mod reward;
pub mod time_range;
pub mod user;

pub trait Validator {
    fn validate(&self) -> StdResult<()>;
}
