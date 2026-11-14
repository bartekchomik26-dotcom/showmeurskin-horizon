// Open the armour config, toggle the mod.
fn smys_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    crate::game::showmeyourskin::key(game, key, pressed);
}
