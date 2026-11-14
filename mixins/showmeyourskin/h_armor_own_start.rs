// Whose armour is laid out next: the local player's.
fn smys_armor_own_start(game: &crate::game::Game<'_>) {
    crate::game::showmeyourskin::armor_start(game, None);
}
