use cw20::Balance;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use cosmwasm_std::{Addr, Coin, StdResult, Storage, Timestamp};
use cw20::Cw20CoinVerified;
use cw_storage_plus::{Item, Map};

use crate::error;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug, Default)]
pub struct AdminList {
    pub admins: Vec<Addr>,
    pub mutable: bool,
}

impl AdminList {
    /// returns true if the address is a registered admin
    pub fn is_admin(&self, addr: impl AsRef<str>) -> bool {
        let addr = addr.as_ref();
        self.admins.iter().any(|a| a.as_ref() == addr)
    }

    /// returns true if the address is a registered admin and the config is mutable
    pub fn can_modify(&self, addr: &str) -> bool {
        self.mutable && self.is_admin(addr)
    }
}

pub const ADMIN_LIST: Item<AdminList> = Item::new("admin_list");
// pub const WHITELIST: Map<String, Option<Timestamp>> = Map::new("white_list");
pub const WHITEHASH: Item<HashMap<String, u64>> = Item::new("white_hash");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_admin() {
        let admins: Vec<_> = vec!["bob", "paul", "john"]
            .into_iter()
            .map(Addr::unchecked)
            .collect();
        let config = AdminList {
            admins: admins.clone(),
            mutable: false,
        };

        assert!(config.is_admin(admins[0].as_ref()));
        assert!(config.is_admin(admins[2].as_ref()));
        assert!(!config.is_admin("other"));
    }

    #[test]
    fn can_modify() {
        let alice = Addr::unchecked("alice");
        let bob = Addr::unchecked("bob");

        // admin can modify mutable contract
        let config = AdminList {
            admins: vec![bob.clone()],
            mutable: true,
        };
        assert!(!config.can_modify(alice.as_ref()));
        assert!(config.can_modify(bob.as_ref()));

        // no one can modify an immutable contract
        let config = AdminList {
            admins: vec![alice.clone()],
            mutable: false,
        };
        assert!(!config.can_modify(alice.as_ref()));
        assert!(!config.can_modify(bob.as_ref()));
    }
}
