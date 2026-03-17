use crate::db::connect;
use crate::models::{Flow, NewFlow, UpdateFlow};
use crate::schema::flows;
use anyhow::{Error, Result};
use diesel::prelude::*;
use serde_json::from_str;

pub fn get_flows() -> Result<Vec<Flow>> {
    let conn = &mut connect()?;
    let results = flows::table.load::<Flow>(conn)?;
    Ok(results)
}

pub fn get_enabled_flows() -> Result<Vec<Flow>> {
    let conn = &mut connect()?;
    let results = flows::table
        .filter(flows::enabled.eq(true))
        .load::<Flow>(conn)?;
    Ok(results)
}

pub fn get_flow(id: i32) -> Result<Flow> {
    let conn = &mut connect()?;
    let flow = flows::table.find(id).first(conn)?;
    Ok(flow)
}

pub fn save_new_flow(payload: String) -> Result<Flow> {
    let conn = &mut connect()?;

    let partial: serde_json::Value = match from_str(&payload) {
        Ok(v) => v,
        Err(e) => {
            return Err(Error::new(e));
        }
    };

    let title = partial["title"].as_str().unwrap_or("Default");
    let graph = partial["graph"].as_str().unwrap_or("{}");
    let flow = NewFlow::new(title, graph);

    diesel::insert_into(flows::table)
        .values(flow)
        .execute(conn)?;

    let new_flow = flows::table.order(flows::id.desc()).first(conn)?;

    Ok(new_flow)
}

pub fn update_flow(payload: String) -> Result<Flow> {
    let conn = &mut connect()?;

    let mut flow = from_str::<UpdateFlow>(&payload)?;
    flow.set_updated_at(chrono::Local::now().naive_local());

    diesel::update(flows::table.find(flow.get_id()))
        .set((
            flows::title.eq(flow.get_title()),
            flows::graph.eq(flow.get_graph()),
            flows::enabled.eq(flow.get_enabled()),
            flows::updated_at.eq(flow.get_updated_at()),
        ))
        .execute(conn)?;

    let updated_flow = flows::table.find(flow.get_id()).first(conn)?;

    Ok(updated_flow)
}

pub fn toggle_flow(id: i32, enabled: bool) -> Result<Flow> {
    let conn = &mut connect()?;

    diesel::update(flows::table.find(id))
        .set((
            flows::enabled.eq(enabled),
            flows::updated_at.eq(chrono::Local::now().naive_local()),
        ))
        .execute(conn)?;

    let flow = flows::table.find(id).first(conn)?;
    Ok(flow)
}

pub fn delete_flow(id: i32) -> Result<()> {
    let conn = &mut connect()?;
    diesel::delete(flows::table.find(id)).execute(conn)?;
    Ok(())
}
