// Whose armour is laid out next: another player's.
fn smys_armor_worn_start(game: &crate::game::Game<'_>, mob: &horizon_client::mobs::Mob) {
    crate::game::showmeyourskin::armor_start(game, Some(mob));
}
