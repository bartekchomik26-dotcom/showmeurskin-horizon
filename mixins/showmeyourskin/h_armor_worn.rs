// The local player's armour layers, as its config shows them.
fn smys_armor_worn(value: Vec<(&Part, [f32; 4], u32, u32, bool)>) -> Vec<(&Part, [f32; 4], u32, u32, bool)> {
    crate::game::showmeyourskin::apply_armor(value)
}
