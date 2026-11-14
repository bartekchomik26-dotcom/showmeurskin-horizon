// The mod in Options > Mods, with its keys.
fn smys_list(mods: &mut Vec<horizon_client::mod_api::NativeMod>) {
    crate::game::showmeyourskin::listed(mods);
}
