# Migrating to Tnua 0.13

- Instead of using the `rapier_2d` or the `rapier_3d` features, add the
  `bevy-tnua-rapier2d` or the `bevy-tnua-rapier3d` crate.
- The `TnuaRapier2d*` or `TnuaRapier3d*` components are no longer in
  `bevy_tnua::prelude`. They should be imported from the new crates.

# Migrating to Tnua 0.10

## Basic ECS initialization

* Instead of `TnuaPlatformerPlugin`, add `TnuaControllerPlugin`.
* Instead of `TnuaPlatformerBundle`, add `TnuaControllerBundle`. Note that
  `TnuaControllerBundle` does not need to be configured - the entire
  configuration is passed with the basis and the actions.

## Character control

Controls should still be passed every frame, preferably
`in_set(TnuaUserControlsSystemSet)`. Instead of using the
`TnuaPlatformerControls` component, the controls should be passed to the
`TnuaController` component.

### Base movement

Instead of changing `TnuaPlatformerControls::desired_velocity`, pass the
`TnuaBuiltinWalk` basis to `TnuaController` and set the `desired_velocity`
there:
```rust
controller.basis(TnuaBuiltinWalk {
    desired_velocity: the_desired_velocity,
    float_height: 2.0, // must be passed as well in this new scheme
    ..Default::default()
});
```
* Note that there is no longer a separate `full_speed` configuration field.
  `TnuaBuiltinWalk::desired_velocity` represents the speed as well as the
  direction.

### Jumping

Instead of changing `TnuaPlatformerControls::jump`, use the `TnuaBuiltinJump`
action:
```rust
if should_jump {
    controller.action(TnuaBuiltinJump {
        height: 4.0,
        ..Default::default()
    });
}
```
* Note that there is no longer a separate `full_jump_height` configuration
  field. `TnuaBuiltinJump::height`, which is part of the action and gets passed
  on each frame, controls it.
* When the action is no longer fed, the jump is stopped. There is no need to
  manually nullify the command.

### Crouching

To crouch, instead of changing the
`TnuaPlatformerControls::float_height_offset`, use the `TnuaBuiltinCrouch`
action.
```rust
if should_crouch {
    controller.action(TnuaBuiltinCrouch {
        float_offset: -0.9,
        ..Default::default()
    });
}
```
* Instead of `TnuaKeepCrouchingBelowObstacles`, use `TnuaCrouchEnforcer`. See
  its documentation regarding its usage.

### Turning

Tnua no longer turns the character automatically according to the movement
direction. `TnuaBuiltinWalk::desired_forward` must be passed manually in order
to make the character turn.

`TnuaManualTurningOutput` was completely removed, without being replaced by
anything. There is no longer a reason for Tnua to manage this kind of manual
turning. Just use some tweening plugin.

The forward direction is **always** assumed to be the negative Z axis. If there
is some other forward direction, the game must do the math itself and set
`desired_forward` to set the negative Z axis of the model to the correct
direction.

## Animating

`TnuaPlatformerAnimatingOutput` is removed. Instead, the animating data should
be taken directly from the `TnuaController`.

The state of the basis can be retrieved with `TnuaController::concrete_basis`:

```rust
let Some((basis_input, basis_state)) = controller.concrete_basis::<TnuaBuiltinWalk>()
else {
    // in case some other basis is used - usually happens before the first basis is fed.
    continue;
};

// basis_input is the `TnuaBuiltinWalk` command, which is usually not very interesting.

// This replaces `let speed = animating_output.running_velocity.length();`:
let speed = basis_state.running_velocity.length();
```

Many games would only ever use one basis (probably `TnuaBuiltinWalk`), but a
game that uses multiple bases (e.g. - different bases for walking and for
swiming) can retrieve the name of the current basis by using the
`TnuaController::basis_name` method. This returns a string that can be
`match`ed on, which should be faster than trying multiple downcasts with
`concrete_basis`.

Actions, similarly, have `TnuaController::concrete_action` and
`TnuaController::action_name` (which is more useful than `basis_name`, because
it should be more common for games to use multiple actions, and because action
may play animation based on the action name only, regardless of the action
state)

For both bases and actions, when matching on `basis_name`/`action_name`, prefer
using the `NAME` constant of the basis/action over a string literal:

```rust
match controller.action_name() {
    Some(TnuaBuiltinJump::NAME) => { /* ... */ }
    Some(TnuaBuiltinCrouch::NAME) => { /* ... */ }
    Some(other) => panic!("Unknown action {other}"),
    None => { /* ... */ }
}
```

Of course, when using `named_basis`/`named_action` it is perfectly acceptable
to use the string literal.

## Storing configuration as data

There is no longer `TnuaPlatformerConfig` component, but it may still be
desirable to store a character's movement configuration in the ECS. The
simplest way to do this is to store the basis and the actions themselves in the
ECS, and clone them into the controller when they are needed.

In the examples, this is done by declaring a new component:
```rust
#[derive(Component)]
pub struct CharacterMotionConfigForPlatformerExample {
    pub speed: f32,
    pub walk: TnuaBuiltinWalk,
    pub jump: TnuaBuiltinJump,
    pub crouch: TnuaBuiltinCrouch,
}
```

which is then added to the character entity:

```rust
cmd.insert(CharacterMotionConfigForPlatformerExample {
    speed: 40.0,
    walk: TnuaBuiltinWalk {
        float_height: 2.0,
        ..Default::default()
    },
    jump: TnuaBuiltinJump {
        height: 4.0,
        ..Default::default()
    },
    crouch: TnuaBuiltinCrouch {
        float_offset: -0.9,
        ..Default::default()
    },
});
```

Note that since `TnuaBuiltinWalk::desired_velocity` is a vector, and the
configuration should only store its magnitude - not direction - we store the
`speed` as a separate field.

Then, in the controls system, the basis and actions can simply be `clone`d:

```rust
controller.basis(TnuaBuiltinWalk {
    desired_velocity: direction * config.speed,
    ..config.walk.clone()
});

if crouch {
    controller.action(config.crouch.clone());
}

if jump {
    controller.action(config.jump.clone());
}
```
