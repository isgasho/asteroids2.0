use super::*;
use crate::gui::*;
use physics::*;

#[derive(Default)]
pub struct GUISystem;

impl<'a> System<'a> for GUISystem {
    type SystemData = (
        (
            Entities<'a>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, Lifes>,
            ReadStorage<'a, Shield>,
            ReadStorage<'a, SideBulletAbility>,
            ReadStorage<'a, DoubleCoinsAbility>,
            ReadStorage<'a, DoubleExpAbility>,
            WriteStorage<'a, ShipStats>,
            WriteStorage<'a, ShotGun>,
            WriteStorage<'a, Isometry>,
            WriteStorage<'a, Velocity>,
            WriteStorage<'a, Spin>,
            ReadStorage<'a, PhysicsComponent>,
            ReadExpect<'a, red::Viewport>,
        ),
        ReadExpect<'a, DevInfo>,
        Write<'a, UI>,
        Read<'a, Progress>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, SpawnedUpgrades>,
        Read<'a, CurrentWave>,
        ReadExpect<'a, Pallete>,
        ReadExpect<'a, MacroGame>,
        ReadExpect<'a, PreloadedSounds>,
        Write<'a, EventChannel<Sound>>,
        WriteExpect<'a, Touches>,
        Write<'a, World<f32>>,
        Write<'a, EventChannel<InsertEvent>>,
        Write<'a, AppState>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                character_markers,
                lifes,
                shields,
                side_bullet_abilities,
                double_coins_abilities,
                double_exp_abilities,
                mut ships_stats,
                mut shotguns,
                isometries,
                mut velocities,
                mut spins,
                physics,
                viewport,
            ),
            // preloaded_particles,
            dev_info,
            mut ui,
            progress,
            preloaded_images,
            spawned_upgrades,
            current_wave,
            pallete,
            macro_game,
            preloaded_sounds,
            mut sounds_channel,
            touches,
            mut world,
            mut insert_channel,
            mut app_state,
        ) = data;
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let d = (w * w + h * h).sqrt();
        //contorls
        #[cfg(any(target_os = "android"))]
        let stick_size = w / 80.0;
        #[cfg(any(target_os = "android"))]
        let ctrl_size = stick_size * 10.0;
        #[cfg(any(target_os = "android"))]
        let move_controller = VecController::new(
            Point2::new(ctrl_size, h - ctrl_size),
            ctrl_size,
            stick_size,
            preloaded_images.circle,
        );
        #[cfg(any(target_os = "android"))]
        let attack_controller = VecController::new(
            Point2::new(w - ctrl_size, h - ctrl_size),
            ctrl_size,
            stick_size,
            preloaded_images.circle,
        );
        let (character, ship_stats, _) = if let Some(value) =
            (&entities, &mut ships_stats, &character_markers)
                .join()
                .next()
        {
            value
        } else {
            return;
        };
        // move controller
        #[cfg(target_os = "android")]
        {
            let mut controlling = touches.iter().any(|x| x.is_some());
            if let Some(dir) = move_controller.set(0, &mut ui, &touches) {
                let (character, _) =
                    (&entities, &character_markers).join().next().unwrap();
                let (_character_isometry, mut character_velocity) = {
                    let character_body = world
                        .rigid_body(physics.get(character).unwrap().body_handle)
                        .unwrap();
                    (*character_body.position(), *character_body.velocity())
                };
                let time_scaler =
                    normalize_60frame(TRACKER.lock().unwrap().last_delta());
                let mut thrust =
                    ship_stats.thrust_force * Vector3::new(dir.x, dir.y, 0.0);
                thrust = thrust_calculation(
                    ship_stats.maneuverability.unwrap(),
                    thrust,
                    *character_velocity.as_vector(),
                );
                *character_velocity.as_vector_mut() += thrust;
                let character_body = world
                    .rigid_body_mut(physics.get(character).unwrap().body_handle)
                    .unwrap();
                character_body.set_velocity(character_velocity);
            }

            if let Some(dir) = attack_controller.set(1, &mut ui, &touches) {
                for (iso, _vel, spin, _char_marker) in (
                    &isometries,
                    &mut velocities,
                    &mut spins,
                    &character_markers,
                )
                    .join()
                {
                    let player_torque = DT
                        * calculate_player_ship_spin_for_aim(
                            dir,
                            iso.rotation(),
                            spin.0,
                        );
                    spin.0 += player_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                }

                let dir = dir.normalize();
                let shotgun = shotguns.get_mut(character);
                if let Some(shotgun) = shotgun {
                    if shotgun.shoot() {
                        let isometry = *isometries.get(character).unwrap();
                        let position = isometry.0.translation.vector;
                        // let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                        let velocity_rel = shotgun.bullet_speed * dir;
                        let char_velocity = velocities.get(character).unwrap();
                        let projectile_velocity = Velocity::new(
                            char_velocity.0.x + velocity_rel.x,
                            char_velocity.0.y + velocity_rel.y,
                        );
                        sounds_channel.single_write(Sound(
                            preloaded_sounds.shot,
                            Point2::new(position.x, position.y),
                        ));
                        let rotation = Rotation2::rotation_between(
                            &Vector2::new(0.0, 1.0),
                            &dir,
                        );
                        let bullets = shotgun.spawn_bullets(
                            EntityType::Player,
                            isometries.get(character).unwrap().0,
                            shotgun.bullet_speed,
                            shotgun.bullets_damage,
                            velocities.get(character).unwrap().0,
                            character,
                        );
                        insert_channel.iter_write(bullets.into_iter());
                    }
                }
            }
        }
        // FPS
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Text(Text {
                position: Point2::new(w / 7.0, h / 20.0),
                text: format!("FPS: {}", dev_info.fps).to_string(),
                color: (1.0, 1.0, 1.0, 1.0),
                font_size: 1.0,
            }),
            with_projection: false,
        });

        // stats
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Text(Text {
                position: Point2::new(w - w / 7.0, h / 20.0),
                text: format!("Score: {}", progress.score).to_string(),
                color: (1.0, 1.0, 1.0, 1.0),
                font_size: 1.0,
            }),
            with_projection: false,
        });

        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Text(Text {
                position: Point2::new(w - w / 7.0, h / 20.0),
                text: format!("$ {}", macro_game.coins).to_string(),
                color: (1.0, 1.0, 1.0, 1.0),
                font_size: 1.0,
            }),
            with_projection: false,
        });

        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Text(Text {
                position: Point2::new(w - w / 7.0, h / 7.0 + h / 20.0),
                text: format!("Wave: {}", current_wave.id).to_string(),
                color: (1.0, 1.0, 1.0, 1.0),
                font_size: 1.0,
            }),
            with_projection: false,
        });

        let side_bullets_cnt = side_bullet_abilities.count();
        let double_coins_cnt = double_coins_abilities.count();
        let double_exp_cnt = double_exp_abilities.count();
        let icon_size = w / 20.0;
        struct Ability {
            pub icon: AtlasImage,
            pub text: String,
        };
        let mut abilities = vec![];

        if double_coins_cnt > 0 {
            let ability = Ability {
                icon: preloaded_images.double_coin,
                text: format!("x{}", double_coins_cnt).to_string(),
            };
            abilities.push(ability);
        }
        if double_exp_cnt > 0 {
            let ability = Ability {
                icon: preloaded_images.double_exp,
                text: format!("x{}", double_exp_cnt).to_string(),
            };
            abilities.push(ability);
        }
        if side_bullets_cnt > 0 {
            let ability = Ability {
                icon: preloaded_images.side_bullet_ability,
                text: format!("+{}", side_bullets_cnt).to_string(),
            };
            abilities.push(ability);
        }

        for (i, ability) in abilities.iter().enumerate() {
            let x_pos = w - w / 7.0;
            let y_pos = (i as f32 + 1.0) * h / 7.0 + h / 20.0;
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Picture(Picture {
                    position: Point2::new(x_pos, y_pos),
                    width: icon_size,
                    height: icon_size,
                    image: ability.icon,
                }),
                with_projection: false,
            });
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(
                        x_pos + 2.0 * icon_size,
                        y_pos + icon_size / 2.0,
                    ),
                    text: ability.text.clone(),
                    color: (1.0, 1.0, 1.0, 1.0),
                    font_size: 1.0,
                }),
                with_projection: false,
            });
        }

        let (_character, _) =
            (&entities, &character_markers).join().next().unwrap();
        // "UI" things
        // experience and level bars
        let experiencebar_w = w / 5.0;
        let experiencebar_h = h / 100.0;
        let experience_position =
            Point2::new(w / 2.0 - experiencebar_w / 2.0, h - h / 20.0);
        let experience_bar = Rectangle {
            position: experience_position,
            width: (progress.experience as f32
                / progress.current_max_experience() as f32)
                * experiencebar_w,
            height: experiencebar_h,
            color: pallete.experience_color.clone(),
        };

        let border = d / 200f32;
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Picture(Picture {
                position: experience_position
                    + Vector2::new(-border / 2.0, -border / 2.0),
                width: experiencebar_w + border,
                height: experiencebar_h + border,
                image: preloaded_images.bar,
            }),
            with_projection: false,
        });
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Picture(Picture {
                position: experience_position
                    + Vector2::new(-border / 2.0, -border / 2.0),
                width: (progress.experience as f32
                    / progress.current_max_experience() as f32)
                    * experiencebar_w,
                height: experiencebar_h,
                image: preloaded_images.bar,
            }),
            with_projection: false,
        });
        ui.primitives.push(Primitive {
            kind: PrimitiveKind::Rectangle(experience_bar),
            with_projection: false,
        });
        if spawned_upgrades.len() > 0 {
            // {
            let (upgrade_bar_w, upgrade_bar_h) = (w / 3f32, h / 10.0);
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Picture(Picture {
                    position: Point2::new(
                        w / 2.0 - upgrade_bar_w / 2.0,
                        h - h / 20.0 - upgrade_bar_h,
                    ),
                    width: upgrade_bar_w,
                    height: upgrade_bar_h,
                    image: preloaded_images.upg_bar,
                }),
                with_projection: false,
            });
        }
        let (lifebar_w, lifebar_h) = (w / 4f32, h / 50.0);
        let health_y = h / 40.0;
        let shields_y = health_y + h / 13.0;
        for (life, shield, _character) in
            (&lifes, &shields, &character_markers).join()
        {
            {
                // upgrade bar
                let border = d / 200f32;
                let (health_back_w, health_back_h) =
                    (lifebar_w + border, lifebar_h + border);
                ui.primitives.push(Primitive {
                    kind: PrimitiveKind::Picture(Picture {
                        position: Point2::new(
                            w / 2.0 - health_back_w / 2.0,
                            health_y - border / 2.0,
                        ),
                        width: health_back_w,
                        height: health_back_h,
                        image: preloaded_images.bar,
                    }),
                    with_projection: false,
                });

                let (health_back_w, health_back_h) =
                    (lifebar_w + border, lifebar_h + border);
                ui.primitives.push(Primitive {
                    kind: PrimitiveKind::Picture(Picture {
                        position: Point2::new(
                            w / 2.0 - health_back_w / 2.0,
                            shields_y - border / 2.0,
                        ),
                        width: health_back_w,
                        height: health_back_h,
                        image: preloaded_images.bar,
                    }),
                    with_projection: false,
                });
            }

            let lifes_bar = Rectangle {
                position: Point2::new(w / 2.0 - lifebar_w / 2.0, health_y),
                width: (life.0 as f32 / ship_stats.max_health as f32)
                    * lifebar_w,
                height: lifebar_h,
                color: pallete.life_color.clone(),
            };
            let shields_bar = Rectangle {
                position: Point2::new(w / 2.0 - lifebar_w / 2.0, shields_y),
                width: (shield.0 as f32 / ship_stats.max_shield as f32)
                    * lifebar_w,
                height: lifebar_h,
                color: pallete.shield_color,
            };
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Rectangle(shields_bar),
                with_projection: false,
            });
            ui.primitives.push(Primitive {
                kind: PrimitiveKind::Rectangle(lifes_bar),
                with_projection: false,
            });
        }
    }
}
