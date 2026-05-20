//! Audits writes to a restricted component.

use bevy::prelude::*;

#[derive(RestrictedAccess)]
struct AccountBalance {
    credits: u32,
}

#[derive(Resource)]
struct AccountEntity(Entity);

#[derive(Resource, Default)]
struct AuditTrail(Vec<String>);

fn main() {
    App::new()
        .init_resource::<AuditTrail>()
        .add_systems(Startup, setup)
        .add_systems(Update, (apply_bonus, verify_audit).chain())
        .run();
}

fn setup(mut commands: Commands) {
    let entity = commands.spawn(AccountBalance { credits: 10 }).id();
    commands.insert_resource(AccountEntity(entity));
}

fn apply_bonus(
    account: Res<AccountEntity>,
    mut balances: RestrictedMut<AccountBalance>,
    mut audit: ResMut<AuditTrail>,
) {
    let (before, after) = balances
        .modify(account.0, |balance| {
            let before = balance.credits;
            balance.credits += 5;
            (before, balance.credits)
        })
        .expect("account should have a balance");

    audit
        .0
        .push(format!("account {:?}: {before} -> {after}", account.0));
}

fn verify_audit(
    account: Res<AccountEntity>,
    balances: Query<&AccountBalance>,
    audit: Res<AuditTrail>,
) {
    let balance = balances
        .get(account.0)
        .expect("account should have a balance");

    assert_eq!(balance.credits, 15);
    assert_eq!(audit.0.len(), 1);
}
