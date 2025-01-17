// Copyright (C) 2023 Lily Lyons
//
// This file is part of Luminol.
//
// Luminol is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Luminol is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Luminol.  If not, see <http://www.gnu.org/licenses/>.
pub use crate::{id, id_vec, optional_id, optional_path, rpg::AudioFile, Path};

#[derive(Default, Debug, serde::Deserialize, serde::Serialize)]
#[serde(rename = "RPG::Skill")]
pub struct Skill {
    #[serde(with = "id")]
    pub id: usize,
    pub name: String,
    #[serde(with = "optional_path")]
    pub icon_name: Path,
    pub description: String,
    pub scope: i32,
    pub occasion: Occasion,
    #[serde(with = "optional_id")]
    pub animation1_id: Option<usize>,
    #[serde(with = "optional_id")]
    pub animation2_id: Option<usize>,
    pub menu_se: AudioFile,
    #[serde(with = "optional_id")]
    pub common_event_id: Option<usize>,
    pub sp_cost: i32,
    pub power: i32,
    pub atk_f: i32,
    pub eva_f: i32,
    pub str_f: i32,
    pub dex_f: i32,
    pub agi_f: i32,
    pub int_f: i32,
    pub hit: i32,
    pub pdef_f: i32,
    pub mdef_f: i32,
    pub variance: i32,
    #[serde(with = "id_vec")]
    pub element_set: Vec<usize>,
    #[serde(with = "id_vec")]
    pub plus_state_set: Vec<usize>,
    #[serde(with = "id_vec")]
    pub minus_state_set: Vec<usize>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]
#[derive(
    num_enum::TryFromPrimitive,
    num_enum::IntoPrimitive,
    strum::Display,
    strum::EnumIter
)]
#[derive(serde::Deserialize, serde::Serialize)]
#[repr(u8)]
#[serde(into = "u8")]
#[serde(try_from = "u8")]
pub enum Occasion {
    #[default]
    Always = 0,
    #[strum(to_string = "Only in battle")]
    OnlyBattle = 1,
    #[strum(to_string = "Only from the menu")]
    OnlyMenu = 2,
    Never = 3,
}
