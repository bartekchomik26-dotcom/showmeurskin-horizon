// A click in the armour config screen.
fn smys_screen_click(
    game: &crate::game::Game<'_>,
    screen: &horizon_client::mod_api::NativeScreenView<'_>,
    button: i32,
    canceled: &mut bool,
) {
    crate::game::showmeyourskin::screen_click(game, screen, button, canceled);
}
