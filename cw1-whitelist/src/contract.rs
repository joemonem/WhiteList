use schemars::JsonSchema;
use std::fmt;
use std::io::Lines;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coins, to_binary, Addr, Api, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Empty, Env,
    MessageInfo, Response, StdResult, SubMsg, Timestamp, WasmMsg,
};

use cw1::CanExecuteResponse;
use cw2::set_contract_version;
use cw20::{Balance, Cw20Coin, Cw20CoinVerified, Cw20ExecuteMsg, Cw20ReceiveMsg};

use std::collections::HashMap;

use crate::error::ContractError;
use crate::msg::{AdminListResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{AdminList, ADMIN_LIST, WHITEHASH};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw1-whitelist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let mut whitelist: HashMap<String, u64> = HashMap::new();
    whitelist.insert(info.sender.into_string(), 100);

    let cfg = AdminList {
        admins: map_validate(deps.api, &msg.admins)?,
        mutable: msg.mutable,
    };
    // ADMIN_LIST.save(deps.storage, &cfg)?;
    WHITEHASH.save(deps.storage, &whitelist)?;
    Ok(Response::default())
}

pub fn map_validate(api: &dyn Api, admins: &[String]) -> StdResult<Vec<Addr>> {
    admins.iter().map(|addr| api.addr_validate(&addr)).collect()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    // Note: implement this function with different type to add support for custom messages
    // and then import the rest of this contract code.
    msg: ExecuteMsg<Empty>,
) -> Result<Response<Empty>, ContractError> {
    match msg {
        ExecuteMsg::Execute { msgs } => execute_execute(deps, env, info, msgs),
        ExecuteMsg::Freeze {} => execute_freeze(deps, env, info),
        ExecuteMsg::UpdateAdmins { admins } => execute_update_admins(deps, env, info, admins),
        ExecuteMsg::Cancel {} => cancel(deps, env, info),
    }
}

pub fn execute_execute<T>(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msgs: Vec<CosmosMsg<T>>,
) -> Result<Response<T>, ContractError>
where
    T: Clone + fmt::Debug + PartialEq + JsonSchema,
{
    if !can_execute(deps.as_ref(), info.sender.as_ref())? {
        Err(ContractError::Unauthorized {})
    } else {
        let res = Response::new()
            .add_messages(msgs)
            .add_attribute("action", "execute");
        Ok(res)
    }
}

pub fn execute_freeze(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let mut cfg = ADMIN_LIST.load(deps.storage)?;
    if !cfg.can_modify(info.sender.as_ref()) {
        Err(ContractError::Unauthorized {})
    } else {
        cfg.mutable = false;
        ADMIN_LIST.save(deps.storage, &cfg)?;

        let res = Response::new().add_attribute("action", "freeze");
        Ok(res)
    }
}

pub fn execute_update_admins(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    admins: Vec<String>,
) -> Result<Response, ContractError> {
    // let mut cfg = ADMIN_LIST.load(deps.storage)?;
    let mut hash = WHITEHASH.load(deps.storage)?;
    let entry_price = coins(100, "UST");
    //exact funds
    if info.funds != entry_price {
        return Err(ContractError::InvalidAmount {});
    }
    //shouldn't already be in the list
    if hash.contains_key(&info.sender.to_string()) == true {
        return Err(ContractError::AlreadySubscribed {});
    } else {
        deps.api.addr_validate(&info.sender.to_string())?;
        hash.insert(info.sender.into_string(), 2629746);
        WHITEHASH.save(deps.storage, &hash)?;
        let res = Response::new().add_attribute("action", "update_admins");
        Ok(res)
    }
}
pub fn cancel(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
    let mut white = WHITEHASH.load(deps.storage)?;
    // should already be in the list
    if white.contains_key(&info.sender.to_string()) == false {
        return Err(ContractError::NotSubscribed {});
    }
    //shouldn't be already expired
    let value = white
        .get(&info.sender.to_string())
        .ok_or(ContractError::NotSubscribed {})?;
    if env.block.time > Timestamp::from_seconds(*value) {
        return Err(ContractError::AlreadyExpired {});
    }

    // let quitter = &info.sender.to_string();
    // // shouldn't be expired
    // for (quitter, value_variable) in white.iter() {
    //     if env.block.time > Timestamp::from_seconds(*value_variable) {
    //         return Err(ContractError::AlreadyExpired {});
    //     }
    // }
    //remove from list
    white.remove(&info.sender.to_string());

    //refund

    let messages = send_tokens(&info.sender, coins(95, "UST"))?;

    Ok(Response::new()
        .add_attribute("action", "refund")
        .add_attribute("to", info.sender)
        .add_submessages(messages))
}

fn send_tokens(to: &Addr, amount: Vec<Coin>) -> StdResult<Vec<SubMsg>> {
    let msgs: Vec<SubMsg> = if amount.is_empty() {
        vec![]
    } else {
        vec![SubMsg::new(BankMsg::Send {
            to_address: to.into(),
            amount: amount.to_vec(),
        })]
    };
    Ok(msgs)
}

// if !cfg.can_modify(info.sender.as_ref()) {
//     Err(ContractError::Unauthorized {})
// } else {
//     cfg.admins = map_validate(deps.api, &admins)?;
//     ADMIN_LIST.save(deps.storage, &cfg)?;

//     let res = Response::new().add_attribute("action", "update_admins");
//     Ok(res)
// }

fn can_execute(deps: Deps, sender: &str) -> StdResult<bool> {
    let cfg = ADMIN_LIST.load(deps.storage)?;
    let can = cfg.is_admin(&sender);
    Ok(can)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::AdminList {} => to_binary(&query_admin_list(deps)?),
        QueryMsg::CanExecute { sender, msg } => to_binary(&query_can_execute(deps, sender, msg)?),
    }
}

pub fn query_admin_list(deps: Deps) -> StdResult<HashMap<String, u64>> {
    // let cfg = ADMIN_LIST.load(deps.storage)?;
    return Ok(WHITEHASH.load(deps.storage)?);

    // Ok(AdminListResponse {
    //     admins: cfg.admins.into_iter().map(|a| a.into()).collect(),
    //     mutable: cfg.mutable,
    // })
}

pub fn query_can_execute(
    deps: Deps,
    sender: String,
    _msg: CosmosMsg,
) -> StdResult<CanExecuteResponse> {
    Ok(CanExecuteResponse {
        can_execute: can_execute(deps, &sender)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coin, coins, BankMsg, StakingMsg, SubMsg, WasmMsg};

    fn mock_env_time(time: u64) -> Env {
        let mut env = mock_env();
        env.block.time = Timestamp::from_seconds(time);
        env
    }

    #[test]
    fn instantiate_and_modify_config() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        let hash: HashMap<String, u64> = HashMap::new();

        let anyone = "anyone";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), bob.to_string(), carl.to_string()],
            mutable: true,
            list: hash,
        };
        let info = mock_info(&anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string(), carl.to_string()],
            mutable: true,
        };
        // assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // anyone cannot modify the contract
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![anyone.to_string()],
        };
        let info = mock_info(&anyone, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but alice can kick out carl
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.to_string(), bob.to_string()],
        };
        let info = mock_info(&alice, &[]);
        execute(deps.as_mut(), mock_env(), info, msg).unwrap();

        // ensure expected config
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: true,
        };
        // assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // carl cannot freeze it
        let info = mock_info(&carl, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {}).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but bob can
        let info = mock_info(&bob, &[]);
        execute(deps.as_mut(), mock_env(), info, ExecuteMsg::Freeze {}).unwrap();
        let expected = AdminListResponse {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: false,
        };
        // assert_eq!(query_admin_list(deps.as_ref()).unwrap(), expected);

        // and now alice cannot change it again
        let msg = ExecuteMsg::UpdateAdmins {
            admins: vec![alice.to_string()],
        };
        let info = mock_info(&alice, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn execute_messages_has_proper_permissions() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let carl = "carl";

        let hash: HashMap<String, u64> = HashMap::new();

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), carl.to_string()],
            mutable: false,
            list: hash,
        };
        let info = mock_info(&bob, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        let freeze: ExecuteMsg<Empty> = ExecuteMsg::Freeze {};
        let msgs = vec![
            BankMsg::Send {
                to_address: bob.to_string(),
                amount: coins(10000, "DAI"),
            }
            .into(),
            WasmMsg::Execute {
                contract_addr: "some contract".into(),
                msg: to_binary(&freeze).unwrap(),
                funds: vec![],
            }
            .into(),
        ];

        // make some nice message
        let execute_msg = ExecuteMsg::Execute { msgs: msgs.clone() };

        // bob cannot execute them
        let info = mock_info(&bob, &[]);
        let err = execute(deps.as_mut(), mock_env(), info, execute_msg.clone()).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});

        // but carl can
        let info = mock_info(&carl, &[]);
        let res = execute(deps.as_mut(), mock_env(), info, execute_msg).unwrap();
        assert_eq!(
            res.messages,
            msgs.into_iter().map(SubMsg::new).collect::<Vec<_>>()
        );
        assert_eq!(res.attributes, [("action", "execute")]);
    }

    #[test]
    fn can_execute_query_works() {
        let mut deps = mock_dependencies();

        let alice = "alice";
        let bob = "bob";
        let hash: HashMap<String, u64> = HashMap::new();

        let anyone = "anyone";

        // instantiate the contract
        let instantiate_msg = InstantiateMsg {
            admins: vec![alice.to_string(), bob.to_string()],
            mutable: false,
            list: hash,
        };
        let info = mock_info(&anyone, &[]);
        instantiate(deps.as_mut(), mock_env(), info, instantiate_msg).unwrap();

        // let us make some queries... different msg types by owner and by other
        let send_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: anyone.to_string(),
            amount: coins(12345, "ushell"),
        });
        let staking_msg = CosmosMsg::Staking(StakingMsg::Delegate {
            validator: anyone.to_string(),
            amount: coin(70000, "ureef"),
        });

        // owner can send
        let res = query_can_execute(deps.as_ref(), alice.to_string(), send_msg.clone()).unwrap();
        assert!(res.can_execute);

        // owner can stake
        let res = query_can_execute(deps.as_ref(), bob.to_string(), staking_msg.clone()).unwrap();
        assert!(res.can_execute);

        // anyone cannot send
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), send_msg).unwrap();
        assert!(!res.can_execute);

        // anyone cannot stake
        let res = query_can_execute(deps.as_ref(), anyone.to_string(), staking_msg).unwrap();
        assert!(!res.can_execute);
    }
    #[test]
    fn try_cancel() {
        let mut deps = mock_dependencies();
        let info = mock_info("jack", &[]);
        let mut hash: HashMap<String, u64> = HashMap::new();
        hash.insert("joe".to_string(), 100);
        let msg = InstantiateMsg {
            admins: vec!["jack".to_string(), "john".to_string()],
            mutable: true,
            list: hash.clone(),
        };
        let res = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        //shouldn't work if non-member cancels
        let info = mock_info("anyone", &[]);
        let err = cancel(deps.as_mut(), mock_env(), info).unwrap_err();
        assert_eq!(err, ContractError::NotSubscribed {});

        //shouldn't be already expired
        let env = mock_env_time(200);
        let info = mock_info("joe", &[]);

        let err = cancel(deps.as_mut(), env, info).unwrap_err();
        assert_eq!(err, ContractError::AlreadyExpired {});

        //make sure the address is removed
        let env = mock_env_time(200);
        let info = mock_info("joe", &[]);

        cancel(deps.as_mut(), env, info.clone()).unwrap();
        assert_eq!(hash.contains_key(&info.sender.to_string()), false);

        //make sure the refund was successful
        assert_eq!(("action", "refund"), res.attributes[0]);
        assert_eq!(1, res.messages.len());
        assert_eq!(
            res.messages[0],
            SubMsg::new(BankMsg::Send {
                to_address: "joe".to_string(),
                amount: coins(95, "UST"),
            })
        );
    }
}
