use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt;

// use crate::factory::PairType;
// use crate::pair::QueryMsg as PairQueryMsg;
// use crate::querier::{query_balance, query_token_balance, query_token_symbol};
use cosmwasm_std::{
    to_binary, Addr, Api, BankMsg, Coin, CosmosMsg, MessageInfo, QuerierWrapper, StdError,
    StdResult, Uint128, WasmMsg,
};
use cw20::Cw20ExecuteMsg;
// use terra_cosmwasm::TerraQuerier;

/// UST token denomination
pub const UUSD_DENOM: &str = "uusd";
/// LUNA token denomination
pub const ULUNA_DENOM: &str = "uluna";

/// ## Description
/// This enum describes a Terra asset (native or CW20).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    /// Information about an asset stored in a [`AssetInfo`] struct
    pub info: AssetInfo,
    /// A token amount
    pub amount: Uint128,
}

impl fmt::Display for Asset {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.amount, self.info)
    }
}

impl Asset {
    /// Returns true if the token is native. Otherwise returns false.
    /// ## Params
    /// * **self** is the type of the caller object.
    pub fn is_native_token(&self) -> bool {
        self.info.is_native_token()
    }

    /// Returns a message of type [`CosmosMsg`].
    ///
    /// For native tokens of type [`AssetInfo`] uses the default method [`BankMsg::Send`] to send a token amount to a recipient.
    /// Before the token is sent, we need to deduct a tax.
    ///
    /// For a token of type [`AssetInfo`] we use the default method [`Cw20ExecuteMsg::Transfer`] and so there's no need to deduct any other tax.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **querier** is an object of type [`QuerierWrapper`]
    ///
    /// * **recipient** is the address where the funds will be sent.
    pub fn into_msg(self, _querier: &QuerierWrapper, recipient: Addr) -> StdResult<CosmosMsg> {
        let amount = self.amount;

        match &self.info {
            AssetInfo::Token { contract_addr } => Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: recipient.to_string(),
                    amount,
                })?,
                funds: vec![],
            })),
            AssetInfo::NativeToken { denom, .. } => Ok(CosmosMsg::Bank(BankMsg::Send {
                to_address: recipient.to_string(),
                amount: vec![Coin {
                    denom: denom.to_string(),
                    amount,
                }],
            })),
        }
    }

    /// Validates an amount of native tokens being sent. Returns [`Ok`] if successful, otherwise returns [`Err`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **message_info** is an object of type [`MessageInfo`]
    pub fn assert_sent_native_token_balance(&self, message_info: &MessageInfo) -> StdResult<()> {
        if let AssetInfo::NativeToken { denom } = &self.info {
            match message_info.funds.iter().find(|x| x.denom == *denom) {
                Some(coin) => {
                    if self.amount == coin.amount {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
                None => {
                    if self.amount.is_zero() {
                        Ok(())
                    } else {
                        Err(StdError::generic_err("Native token balance mismatch between the argument and the transferred"))
                    }
                }
            }
        } else {
            Ok(())
        }
    }
}

/// This enum describes available Token types.
/// ## Examples
/// ```
/// # use cosmwasm_std::Addr;
/// # use margined_common::asset::AssetInfo::{NativeToken, Token};
/// Token { contract_addr: Addr::unchecked("terra...") };
/// NativeToken { denom: String::from("uluna") };
/// ```
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AssetInfo {
    /// Non-native Token
    Token { contract_addr: Addr },
    /// Native token
    NativeToken { denom: String },
}

impl fmt::Display for AssetInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AssetInfo::NativeToken { denom } => write!(f, "{}", denom),
            AssetInfo::Token { contract_addr } => write!(f, "{}", contract_addr),
        }
    }
}

impl AssetInfo {
    /// Returns true if the caller is a native token. Otherwise returns false.
    /// ## Params
    /// * **self** is the caller object type
    pub fn is_native_token(&self) -> bool {
        match self {
            AssetInfo::NativeToken { .. } => true,
            AssetInfo::Token { .. } => false,
        }
    }

    /// Returns True if the calling token is the same as the token specified in the input parameters.
    /// Otherwise returns False.
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **asset** is object of type [`AssetInfo`].
    pub fn equal(&self, asset: &AssetInfo) -> bool {
        match self {
            AssetInfo::Token { contract_addr, .. } => {
                let self_contract_addr = contract_addr;
                match asset {
                    AssetInfo::Token { contract_addr, .. } => self_contract_addr == contract_addr,
                    AssetInfo::NativeToken { .. } => false,
                }
            }
            AssetInfo::NativeToken { denom, .. } => {
                let self_denom = denom;
                match asset {
                    AssetInfo::Token { .. } => false,
                    AssetInfo::NativeToken { denom, .. } => self_denom == denom,
                }
            }
        }
    }

    /// If the caller object is a native token of type ['AssetInfo`] then his `denom` field converts to a byte string.
    ///
    /// If the caller object is a token of type ['AssetInfo`] then his `contract_addr` field converts to a byte string.
    /// ## Params
    /// * **self** is the type of the caller object.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            AssetInfo::NativeToken { denom } => denom.as_bytes(),
            AssetInfo::Token { contract_addr } => contract_addr.as_bytes(),
        }
    }

    /// Returns [`Ok`] if the token of type [`AssetInfo`] is in lowercase and valid. Otherwise returns [`Err`].
    /// ## Params
    /// * **self** is the type of the caller object.
    ///
    /// * **api** is a object of type [`Api`]
    pub fn check(&self, api: &dyn Api) -> StdResult<()> {
        match self {
            AssetInfo::Token { contract_addr } => {
                addr_validate_to_lower(api, contract_addr.as_str())?;
            }
            AssetInfo::NativeToken { denom } => {
                if !denom.starts_with("ibc/") && denom != &denom.to_lowercase() {
                    return Err(StdError::generic_err(format!(
                        "Non-IBC token denom {} should be lowercase",
                        denom
                    )));
                }
            }
        }
        Ok(())
    }
}

/// Returns a lowercased, validated address upon success. Otherwise returns [`Err`]
/// ## Params
/// * **api** is an object of type [`Api`]
///
/// * **addr** is an object of type [`Addr`]
pub fn addr_validate_to_lower(api: &dyn Api, addr: &str) -> StdResult<Addr> {
    if addr.to_lowercase() != addr {
        return Err(StdError::generic_err(format!(
            "Address {} should be lowercase",
            addr
        )));
    }
    api.addr_validate(addr)
}

/// Returns an [`Asset`] object representing a native token and an amount of tokens.
/// ## Params
/// * **denom** is a [`String`] that represents the native asset denomination.
///
/// * **amount** is a [`Uint128`] representing an amount of native assets.
pub fn native_asset(denom: String, amount: Uint128) -> Asset {
    Asset {
        info: AssetInfo::NativeToken { denom },
        amount,
    }
}

/// Returns an [`Asset`] object representing a non-native token and an amount of tokens.
/// ## Params
/// * **contract_addr** is a [`Addr`]. It is the address of the token contract.
///
/// * **amount** is a [`Uint128`] representing an amount of tokens.
pub fn token_asset(contract_addr: Addr, amount: Uint128) -> Asset {
    Asset {
        info: AssetInfo::Token { contract_addr },
        amount,
    }
}

/// Returns an [`AssetInfo`] object representing the denomination for a Terra native asset.
/// ## Params
/// * **denom** is a [`String`] object representing the denomination of the Terra native asset.
pub fn native_asset_info(denom: String) -> AssetInfo {
    AssetInfo::NativeToken { denom }
}

/// Returns an [`AssetInfo`] object representing the address of a token contract.
/// ## Params
/// * **contract_addr** is a [`Addr`] object representing the address of a token contract.
pub fn token_asset_info(contract_addr: Addr) -> AssetInfo {
    AssetInfo::Token { contract_addr }
}
