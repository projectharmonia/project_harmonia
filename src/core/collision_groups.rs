use bevy_rapier3d::prelude::*;

/// Adds meaningful aliases for groups.
pub(super) trait LifescapeGroupsExt {
    const GROUND: Self;
    const ACTOR: Self;
    const OBJECT: Self;
    const WALL: Self;
}

impl LifescapeGroupsExt for Group {
    const GROUND: Self = Group::GROUP_1;
    const ACTOR: Self = Group::GROUP_2;
    const OBJECT: Self = Group::GROUP_3;
    const WALL: Self = Group::GROUP_4;
}
