// The mod's page in Options > Mods.
fn smys_settings(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    crate::game::showmeyourskin::settings_page(game, mod_id, buttons);
}
