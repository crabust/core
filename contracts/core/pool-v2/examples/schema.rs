use std::env::current_dir;
use std::fs::create_dir_all;

use core_pool_v2::state::{config, state};
use cosmwasm_schema::{export_schema, remove_schemas, schema_for};
use pylon_core::pool_msg::{HandleMsg, InitMsg, QueryMsg};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(config::Config), &out_dir);
    export_schema(&schema_for!(state::State), &out_dir);
}
