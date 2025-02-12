use crate::db::connect;
use crate::models::{NewScript, Script, UpdateScript};
use crate::schema::scripts;
use anyhow::{Error, Result};
use diesel::prelude::*;
use serde_json::from_str;

pub fn get_script(id: i32) -> Result<Script> {
    let conn = &mut connect()?;

    let script = scripts::table.find(id).first(conn)?;

    Ok(script)
}

pub fn get_scripts() -> Result<Vec<Script>> {
    let conn = &mut connect()?;

    let scripts = scripts::table.load::<Script>(conn)?;

    Ok(scripts)
}

pub fn save_new_script(payload: String) -> Result<Script> {
    let conn = &mut connect()?;

    let script = match from_str::<NewScript>(&payload) {
        Ok(s) => s,
        Err(e) => {
            return Err(Error::new(e));
        }
    };

    diesel::insert_into(scripts::table)
        .values(script)
        .execute(conn)?;

    let new_script = scripts::table.order(scripts::id.desc()).first(conn)?;

    Ok(new_script)
}

pub fn delete_script(id: i32) -> Result<()> {
    let conn = &mut connect()?;

    diesel::delete(scripts::table.find(id)).execute(conn)?;

    Ok(())
}

pub fn update_script(payload: String) -> Result<Script> {
    let conn = &mut connect()?;

    let script = from_str::<UpdateScript>(&payload)?;

    diesel::update(scripts::table.find(script.get_id()))
        .set((
            scripts::title.eq(&script.get_title()),
            scripts::code.eq(&script.get_code()),
            scripts::schedule.eq(script.get_schedule()),
            scripts::updated_at.eq(script.get_updated_at()),
        ))
        .execute(conn)?;

    let updated_script = scripts::table.find(script.get_id()).first(conn)?;

    Ok(updated_script)
}
