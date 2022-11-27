//here we define what the contract does
//create the contract, write data, read data and then migrate
//CW2 allows users to define contract metadata

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, from_binary, Binary, Deps, DepsMut, Env, MessageInfo, Order, Response, StdResult, Uint128, WasmMsg, BankMsg, coin
};
use cw20::Cw20ReceiveMsg;
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{Cw20DepositResponse, ExecuteMsg, InstantiateMsg, QueryMsg, Cw20HookMsg, DepositResponse};
use crate::state::{Cw20Deposits, CW20_DEPOSITS, DEPOSITS, Deposits};

const CONTRACT_NAME: &str = "deposit-cw20-example"; //metadata
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION"); //metadata: 0.1.0 , this is used when migrate

#[cfg_attr(not(feature = "library"), entry_point)]
//this will be called with instantiate msg
pub fn instantiate(
    deps: DepsMut,
    _env: Env,              //didn't use values we can prefix them
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?; //? it unwraps the value, unwrapping means assume success
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
//write data 
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
    owner:String,
    _contract:String
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit { } => execute_deposit(deps, info),
        ExecuteMsg::Withdraw { amount, denom } => execute_withdraw(deps, info, amount, denom),
        ExecuteMsg::Receive(cw20_msg) => receive_cw20(deps, _env, info, cw20_msg),
        ExecuteMsg::WithdrawCw20 { address, amount } => execute_cw20_withdraw(deps, info, address, amount),
        ExecuteMsg::DepositCw20 { address, amount } => execute_cw20_deposit(deps, info, owner, amount),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Deposits { address } => {
            to_binary(&query_deposits(deps, address)?)
        },
        QueryMsg::Cw20Deposits { address } => to_binary(&query_cw20_deposits(deps, address)?),
    }
}

pub fn receive_cw20(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    cw20_msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    match from_binary(&cw20_msg.msg) {
        Ok(Cw20HookMsg::Deposit { }) => execute_cw20_deposit(deps, info, cw20_msg.sender, cw20_msg.amount),
        _ => Err(ContractError::CustomError { val: "Invalid Cw20HookMsg".to_string() }),
    }
}

pub fn execute_deposit(
    deps: DepsMut,
    info: MessageInfo,
) -> Result<Response, ContractError> {
    let sender = info.sender.clone().into_string(); //it's create the deposit in memory

    let d_coins = info.funds[0].clone();
    
    //check to see if deposit exists
    match DEPOSITS.load(deps.storage, (&sender, d_coins.denom.as_str())) {
        Ok(mut deposit) => {
            //add coins to their account
            deposit.coins.amount += d_coins.amount;
            deposit.coins.amount = deposit.coins.amount.checked_add(d_coins.amount).unwrap();
            deposit.count = deposit.count.checked_add(1).unwrap();
            //map the key and data
            //we can't use ? because it's not return anything 
            //store it on chain
            DEPOSITS.save(deps.storage, (&sender, d_coins.denom.as_str()), &deposit).unwrap();
        }
        Err(_) => {
            //user does not exist, add them.
            let deposit = Deposits {
                count: 1,
                owner: info.sender,
                coins: d_coins.clone(),
            };

            DEPOSITS.save(deps.storage, (&sender, d_coins.denom.as_str()), &deposit).unwrap();
        }
    }

    //it returns it was success
    Ok(Response::new()
        .add_attribute("execute", "deposit")
        .add_attribute("denom", d_coins.denom)
        .add_attribute("amount", d_coins.amount)
    )
}

pub fn execute_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    amount:u128,
    denom:String
) -> Result<Response, ContractError> {

    let sender = info.sender.clone().into_string();

    let mut deposit = DEPOSITS.load(deps.storage, (&sender, denom.as_str())).unwrap();
    deposit.coins.amount = deposit.coins.amount.checked_sub(Uint128::from(amount)).unwrap();
    deposit.count = deposit.count.checked_sub(1).unwrap();
    DEPOSITS.save(deps.storage, (&sender, denom.as_str()), &deposit).unwrap();

    let msg = BankMsg::Send {
        to_address: sender.clone(),
        amount: vec![coin(amount, denom.clone())],
    };

    Ok(Response::new()
        .add_attribute("execute", "withdraw")
        .add_attribute("denom", denom)
        .add_attribute("amount", amount.to_string())
        .add_message(msg)
    )
}

pub fn execute_cw20_deposit(
    deps: DepsMut, 
    info: MessageInfo, 
    owner:String, 
    amount:Uint128
) -> Result<Response, ContractError> {
    let cw20_contract_address = info.sender.clone().into_string();
    //check to see if u
    match CW20_DEPOSITS.load(deps.storage, (&owner, cw20_contract_address.as_str())) {
        Ok(mut deposit) => {
            //add coins to their account
            
            deposit.amount = deposit.amount.checked_add(amount).unwrap();
            deposit.count = deposit.count.checked_add(1).unwrap();
            CW20_DEPOSITS.save(deps.storage, (&owner, cw20_contract_address.as_str()), &deposit).unwrap();
        }
        Err(_) => {
            //user does not exist, add them.
            let deposit = Cw20Deposits {
                contract:cw20_contract_address.clone(),
                amount,
                count: 1,
                owner: owner.clone(),
            };
            CW20_DEPOSITS.save(deps.storage, (&owner, cw20_contract_address.as_str()), &deposit).unwrap();
        }
    }
    Ok(Response::new()
        .add_attribute("execute", "deposit")
        .add_attribute("address", cw20_contract_address)
        
    )
}

//use WasmMsg::Execute instead of BankMsg::Send
pub fn execute_cw20_withdraw(
    deps: DepsMut,
    info: MessageInfo,
    contract:String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let sender = info.sender.clone().into_string();
    match CW20_DEPOSITS.load(deps.storage, (&sender, &contract)) {
        Ok(mut deposit) => {
            
           let mut deposit = DEPOSITS.load(deps.storage, (&sender, contract.as_str())).unwrap();
    deposit.coins.amount = deposit.coins.amount.checked_sub(Uint128::from(amount)).unwrap();
    deposit.count = deposit.count.checked_sub(1).unwrap();
    DEPOSITS.save(deps.storage, (&sender, contract.as_str()), &deposit).unwrap();

    let msg = WasmMsg::Execute { contract_addr: contract, msg: to_binary(&deposit)?, funds: vec![]};
    }

    Err(_) => {
    return Err(ContractError::NoCw20ToWithdraw {  });
    }

}

    Ok(Response::new()
        .add_attribute("execute", "withdraw")
        .add_attribute("amount", amount.to_string())
        
    )
       
    }


//read data 
pub fn query_deposits(deps: Deps, address:String) -> StdResult<DepositResponse> {
    let res: StdResult<Vec<_>> = DEPOSITS.prefix(&address).range(deps.storage, None, None, Order::Ascending).collect();
    let deposits = res?;
    Ok(DepositResponse { deposits })
}

fn query_cw20_deposits(deps: Deps, address: String) -> StdResult<Cw20DepositResponse> {

    let res: StdResult<Vec<_>> = CW20_DEPOSITS.prefix(&address).range(deps.storage, None, None, Order::Ascending).collect();
    let deposits = res?;
    Ok(Cw20DepositResponse { deposits })
}


