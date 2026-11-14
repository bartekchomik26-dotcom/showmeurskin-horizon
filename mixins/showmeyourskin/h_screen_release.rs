// The mouse let go in the armour config screen: the held slider is saved.
fn smys_screen_release(game: &crate::game::Game<'_>, screen: &horizon_client::mod_api::NativeScreenView<'_>, canceled: &mut bool) {
    crate::game::showmeyourskin::screen_release(game, screen, canceled);
}
