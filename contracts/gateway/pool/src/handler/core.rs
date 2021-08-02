use cosmwasm_bignumber::Uint256;
use cosmwasm_std::{log, to_binary, CosmosMsg, HumanAddr, StdError, WasmMsg};
use cosmwasm_std::{Api, Env, Extern, HandleResponse, Querier, StdResult, Storage};
use cw20::Cw20HandleMsg;
use std::ops::{Add, Sub};

use crate::lib_staking as staking;
use crate::state::{config, state, user};

pub fn update<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    target: Option<HumanAddr>,
) -> StdResult<HandleResponse> {
    let config = config::read(&deps.storage)?;
    let applicable_reward_time = std::cmp::min(config.finish_time, env.block.time);

    // reward
    let mut state = state::read(&deps.storage)?;
    state.reward_per_token_stored =
        state
            .reward_per_token_stored
            .add(staking::calculate_reward_per_token(
                deps,
                &state,
                &applicable_reward_time,
            )?);
    state.last_update_time = applicable_reward_time;
    state::store(&mut deps.storage, &state)?;

    // user
    let mut user_update_logs = vec![];
    if let Some(target) = target {
        let t = deps.api.canonical_address(&target)?;

        let mut user = user::read(&deps.storage, &t)?;

        user.reward =
            staking::calculate_rewards(deps, &state, &user, Option::from(applicable_reward_time))?;
        user.reward_per_token_paid = state.reward_per_token_stored;

        user::store(&mut deps.storage, &t, &user)?;

        user_update_logs.append(&mut vec![log("target", target), log("reward", user.reward)]);
    }

    Ok(HandleResponse {
        messages: vec![],
        log: [
            vec![
                log("action", "update"),
                log("sender", env.message.sender),
                log("stored_rpt", state.reward_per_token_stored),
            ],
            user_update_logs,
        ]
        .concat(),
        data: None,
    })
}

pub fn deposit_internal<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    amount: Uint256,
) -> StdResult<HandleResponse> {
    if !env.message.sender.eq(&env.contract.address) {
        return Err(StdError::generic_err(format!(
            "Lockup: cannot execute internal function with unauthorized sender. (sender: {})",
            env.message.sender,
        )));
    }
    let config = config::read(&deps.storage)?;

    // check time range // open_deposit flag
    if env.block.time.lt(&config.start_time) && env.block.time.gt(&config.finish_time) {
        return Err(StdError::generic_err(format!(
            "Lockup: cannot deposit tokens out of period range. (now: {}, starts: {}, ends: {})",
            env.block.time, config.start_time, config.finish_time,
        )));
    }

    let owner = deps.api.canonical_address(&sender)?;
    let mut state = state::read(&deps.storage)?;
    let mut user = user::read(&deps.storage, &owner)?;

    state.total_deposit = state.total_deposit.add(amount);
    user.amount = user.amount.add(amount);

    state::store(&mut deps.storage, &state)?;
    user::store(&mut deps.storage, &owner, &user)?;

    Ok(HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "deposit"),
            log("sender", env.message.sender),
            log("deposit_amount", amount),
        ],
        data: None,
    })
}

pub fn withdraw_internal<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
    amount: Uint256,
) -> StdResult<HandleResponse> {
    if !env.message.sender.eq(&env.contract.address) {
        return Err(StdError::generic_err(format!(
            "Lockup: cannot execute internal function with unauthorized sender. (sender: {})",
            env.message.sender,
        )));
    }

    let config = config::read(&deps.storage)?;

    // check time range // open_withdraw flag
    if env.block.time.lt(&config.finish_time) {
        return Err(StdError::generic_err(format!(
            "Lockup: cannot withdraw tokens during lockup period. (now: {}, ends: {})",
            env.block.time, config.finish_time,
        )));
    }

    let owner = deps.api.canonical_address(&sender)?;
    let mut state = state::read(&deps.storage)?;
    let mut user = user::read(&deps.storage, &owner)?;

    if amount > user.amount {
        return Err(StdError::generic_err(
            "Staking: amount must be smaller than user.amount",
        ));
    }

    state.total_deposit = state.total_deposit.sub(amount);
    user.amount = user.amount.sub(amount);

    state::store(&mut deps.storage, &state)?;
    user::store(&mut deps.storage, &owner, &user)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.share_token)?,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: sender.clone(),
                amount: amount.into(),
            })?,
            send: vec![],
        })],
        log: vec![
            log("action", "withdraw"),
            log("sender", sender),
            log("withdraw_amount", amount),
        ],
        data: None,
    })
}

pub fn claim_internal<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    sender: HumanAddr,
) -> StdResult<HandleResponse> {
    if !env.message.sender.eq(&env.contract.address) {
        return Err(StdError::generic_err(format!(
            "Lockup: cannot execute internal function with unauthorized sender. (sender: {})",
            env.message.sender,
        )));
    }

    let config = config::read(&deps.storage)?;

    // check time range // open_claim flag
    if env.block.time.lt(&config.cliff_time) {
        return Err(StdError::generic_err(format!(
            "Lockup: cannot claim rewards during lockup period. (now: {}, ends: {})",
            env.block.time, config.cliff_time
        )));
    }

    let owner = deps.api.canonical_address(&sender)?;
    let mut user = user::read(&deps.storage, &owner)?;

    let claim_amount = user.reward;
    user.reward = Uint256::zero();

    state::store_user(&mut deps.storage, &owner, &user)?;

    let config = config::read(&deps.storage)?;

    Ok(HandleResponse {
        messages: vec![CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: deps.api.human_address(&config.reward_token)?,
            msg: to_binary(&Cw20HandleMsg::Transfer {
                recipient: sender.clone(),
                amount: claim_amount.into(),
            })?,
            send: vec![],
        })],
        log: vec![
            log("action", "claim"),
            log("sender", sender),
            log("claim_amount", claim_amount),
        ],
        data: None,
    })
}
