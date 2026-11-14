// The wheel in the armour config screen (the player list).
fn smys_screen_scroll(game: &crate::game::Game<'_>, scroll: [f64; 2], canceled: &mut bool) {
    crate::game::showmeyourskin::screen_scroll(game, scroll, canceled);
}
