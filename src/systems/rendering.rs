use gfx_h::{TextData, RenderMode};
use gfx_h::effects::MenuParticles;
use std::collections::{HashMap};
use telemetry::{TeleGraph, render_plot};
use num_enum::TryFromPrimitive;
use std::convert::TryFrom;
pub use crate::gui::{Button, Rectangle, Picture, Selector};

// use flame;

use super::*;
#[cfg(any(target_os = "android"))]
use crate::gui::VecController;
use glyph_brush::{Section, rusttype::Scale};
use crate::geometry::{shadow_geometry};

const BUTTON_SCALE: f32 = 1.2;

#[derive(Clone, Copy, Debug, TryFromPrimitive)]
#[repr(usize)]
enum Widgets {
    BackMenu,
    LazerGun, 
    BlasterGun, 
    ShotGun,
    BasicShip,
    HeavyShip,
    LockedBasicShip,
    LockedHeavyShip,
    ScoreTable,
    Play,
    Upgrade1,
    Upgrade2,
    Upgrade,
    Done,
    WeaponSelector,
    ShipsSelector,
}

pub struct ScoreTableRendering {
    reader: ReaderId<Primitive>,
}

impl ScoreTableRendering {
    pub fn new(reader: ReaderId<Primitive>) -> Self {
        ScoreTableRendering {
            reader: reader
        }
    }
}

impl<'a> System<'a> for ScoreTableRendering {
    type SystemData = (
        ReadStorage<'a, ThreadPin<ImageData>>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, red::Viewport>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, UI>,
        Read<'a, Mouse>,
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
        Write<'a, AppState>,
        ReadExpect<'a, MacroGame>
    );
    fn run(&mut self, data: Self::SystemData) {
        let (
            image_datas,
            gl,
            mut canvas,
            viewport,
            mut primitives_channel,
            mut ui,
            mouse,
            mut text_data,
            mut app_state,
            macro_game,
        ) = data;
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_color();
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let (button_w, button_h) = (w/4f32, h/4f32);

        let mut current_h = h / 20.0;
        let text_gap_h = h / 20.0; // TODO somehow measure it
        for score in macro_game.score_table.iter() {
            current_h += text_gap_h;
            ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Text(Text {
                        position: Point2::new(w/20.0, current_h), 
                        text: format!("{}", score).to_string(), 
                    }),
                    with_projection: false,
                }
            );            
        }


        let mut back_to_menu = Button::new(
            Point2::new(w / 2.0, 1.5 * button_h),
            button_w,
            button_h,
            Some(Point3::new(0f32, 0f32, 0f32)),
            false,
            None,
            "Back to Menu".to_string(),
            Widgets::BackMenu as usize
        );
        if back_to_menu.place_and_check(&mut ui, &*mouse) {
            *app_state = AppState::Menu;
        }

        primitives_channel.iter_write(ui.primitives.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &image_datas,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
        );

    }
}
pub struct MenuRenderingSystem {
    reader: ReaderId<Primitive>,
}

impl MenuRenderingSystem {
    pub fn new(reader: ReaderId<Primitive>) -> Self {
        MenuRenderingSystem{
            reader: reader
        }
    }
}

impl<'a> System<'a> for MenuRenderingSystem {
    type SystemData = (
        ReadStorage<'a, ThreadPin<ImageData>>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, red::Viewport>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, UI>,
        Write<'a, UIState>,
        Write<'a, EventChannel<InsertEvent>>,
        WriteExpect<'a, PreloadedImages>,
        WriteExpect<'a, ThreadPin<MenuParticles>>,
        Read<'a, Mouse>,
        Write<'a, AppState>,
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
        ReadExpect<'a, Description>,
        Read<'a, Vec<UpgradeCardRaw>>,
        Write<'a, Vec<UpgradeCard>>,
        Read<'a, HashMap<String, specs::Entity>>,
        WriteExpect<'a, MacroGame>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            image_datas,
            gl,
            mut canvas,
            viewport,
            mut primitives_channel,
            mut ui,
            mut ui_state,
            mut insert_channel,
            preloaded_images,
            mut menu_particles,
            mouse,
            mut app_state,
            // text_data
            mut text_data,
            description,
            upgrade_cards_raw,
            mut avaliable_upgrades,
            name_to_image,
            mut macro_game
        ) = data;
        let mut frame = red::Frame::new(&gl);
        frame.set_clear_color(0.0, 0.0, 0.0, 1.0);
        frame.clear_color();
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        // return;

        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w - w/7.0, h / 20.0), 
                    text: format!(
                        "$ {}", 
                        macro_game.coins
                    ).to_string()
                }),
                with_projection: false,
            }
        );

        let button_w = w/12f32;
        let button_h = button_w;
        let mut buttons = vec![];
        let buttons_names = vec!["", "", ""];
        let guns = vec![Widgets::LazerGun, Widgets::BlasterGun, Widgets::ShotGun];
        let buttons_num = buttons_names.len();
        let button_images = vec![
            preloaded_images.lazer, 
            preloaded_images.blaster, 
            preloaded_images.shotgun
        ];
        let shift_between = w / 20f32;
        let shift_init = w / 2.0 - shift_between - button_w - button_w / 2.0; 
                // -button_w / 2.0 since start draw from left corner :)
        for i in 0..buttons_num {
            let button = Button::new(
                Point2::new(
                    shift_init + i as f32 * (shift_between + button_w), 
                    button_h / 2f32
                ),
                button_w,
                button_h,
                Some(Point3::new(0f32, 0f32, 0f32)),
                false,
                Some(Image(button_images[i])),
                buttons_names[i].to_string(),
                guns[i] as usize
            );
            buttons.push(button);
        }
        let weapon_selector = Selector {
            buttons: buttons,
            id: Widgets::WeaponSelector as usize,
            mask: None
        };
        if let Some(selected_id) = weapon_selector.place_and_check(
            &mut ui,
            &*mouse
        ) {
            match Widgets::try_from(selected_id).expect("unknown widget id") {
                Widgets::LazerGun => {
                    ui_state.chosed_gun = Some(description.player_guns[0].clone());
                }
                Widgets::BlasterGun => {
                    ui_state.chosed_gun = Some(description.player_guns[1].clone());
                }
                Widgets::ShotGun => {
                    ui_state.chosed_gun = Some(description.player_guns[2].clone());
                }
                _ => ()
            }
        }
        let mut buttons = vec![];
        let ships_ids = vec![Widgets::BasicShip, Widgets::HeavyShip];
        let locked_ships_ids = vec![Widgets::LockedBasicShip, Widgets::LockedHeavyShip];
        for (i, ship) in description.player_ships.iter().enumerate() {
            let unlocked = macro_game.ships_unlocked[i];
            let button_position = 
                Point2::new(
                    shift_init + i as f32 * (shift_between + button_w), 
                    button_h + button_h
                );
            if unlocked {
                let button = Button::new(
                    button_position,
                    button_w,
                    button_h,
                    Some(Point3::new(0f32, 0f32, 0f32)),
                    false,
                    Some(ship.image),
                    "".to_string(),
                    ships_ids[i] as usize,
                );
                buttons.push(button);
            } else {
                let button = Button::new(
                    button_position,
                    button_w,
                    button_h,
                    Some(Point3::new(0f32, 0f32, 0f32)),
                    false,
                    Some(ship.image),
                    format!("{} $", description.ship_costs[i]),
                    locked_ships_ids[i] as usize,
                );
                buttons.push(button);
            }
        }

        let ships_selector = Selector {
            buttons: buttons,
            id: Widgets::ShipsSelector as usize,
            mask: Some(macro_game.ships_unlocked.clone())
        };
        if let Some(selected_id) = ships_selector.place_and_check(
            &mut ui,
            &* mouse
        ) {
            // ui_state.chosed_ship = Some(selected_id);
            match Widgets::try_from(selected_id).expect("unknown widget id") {
                Widgets::BasicShip => {
                    ui_state.chosed_ship = Some(0);
                }
                Widgets::HeavyShip => {
                    ui_state.chosed_ship = Some(1);
                }
                Widgets::LockedHeavyShip => {
                    if macro_game.coins >= description.ship_costs[1] {
                        macro_game.ships_unlocked[1] = true;
                        macro_game.coins -= description.ship_costs[1];
                    }
                }
                _ => ()
            }
        }
        // for i in 0..buttons.len() {
        //     if buttons[i].place_and_check(&mut ui, &*mouse) {
        //         dbg!(&format!("ship button {}", i));
        //     }
        // }

        let button_w = w / 6.0;
        let button_h = button_w;
        let score_table_button = Button::new(
            Point2::new(w / 2.0, 1.5 * button_h + shift_between),
            button_w,
            button_h / 5.0,
            Some(Point3::new(0f32, 0f32, 0f32)),
            false,
            Some(Image(preloaded_images.upg_bar)),
            "Score Table".to_string(),
            Widgets::ScoreTable as usize
        );
        if score_table_button.place_and_check(&mut ui, &*mouse) {
            *app_state = AppState::ScoreTable;
        }
        menu_particles.update(0.5);
        canvas
            .render_instancing(
                &gl,
                &viewport,
                &mut frame,
                &menu_particles.instancing_data,
                &Isometry3::new(
                    Vector3::new(0f32, 0f32, 0f32),
                    Vector3::new(0f32, 0f32, 0f32),
                )
            );
        let button_w = button_w / 2.0;
        let button_h = button_w;
        let button = Button::new(
            Point2::new(w/2.0 - button_w / 2.0, h - button_h), 
            // Point2::new(0f32, 0f32),
            button_w, 
            button_h, 
            None,
            false, 
            Some(Image(preloaded_images.play)),
            "".to_string(),
            Widgets::Play as usize
        );
        if let (Some(ship), Some(gun)) = (ui_state.chosed_ship.clone(), ui_state.chosed_gun.clone()) {
            if button.place_and_check(&mut ui, &*mouse) {
                *app_state = AppState::Play(PlayState::Action);
                insert_channel.single_write(InsertEvent::Character{ 
                    gun_kind: gun.clone(), 
                    ship_stats: description.player_ships[ship].ship_stats
                });
                ui_state.chosed_gun = None;
                *avaliable_upgrades = get_avaliable_cards(
                    &upgrade_cards_raw,
                    &gun.clone(),
                    &name_to_image
                );
            }
        }
        primitives_channel.iter_write(ui.primitives.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &image_datas,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
        );
    }
}

pub fn render_primitives<'a>(
    mouse: &Read<'a, Mouse>,
    reader: &mut ReaderId<Primitive>,
    frame: &mut red::Frame,
    image_datas: &ReadStorage<'a, ThreadPin<ImageData>>,
    gl: &ReadExpect<'a, ThreadPin<red::GL>>,
    canvas: &mut WriteExpect<'a, Canvas>,
    viewport: &ReadExpect<'a, red::Viewport>,
    primitives_channel: &mut Write<'a, EventChannel<Primitive>>,
    text_data: &mut WriteExpect<'a, ThreadPin<TextData<'static>>>,
) {
    let dims = viewport.dimensions();
    let (w, h) = (dims.0 as f32, dims.1 as f32);
    let scale = Scale::uniform(((w * w + h * h).sqrt() / 11000.0 * mouse.hdpi as f32).round());
    for primitive in primitives_channel.read(reader) {
        match primitive {
            Primitive {
                kind: PrimitiveKind::Picture(picture),
                with_projection,
            } => {
                let (model, _points, _indicies) = picture.get_gfx();
                canvas
                    .render_primitive_texture(
                        &gl, 
                        &viewport,
                        frame, 
                        image_datas.get(picture.image.0).unwrap(),
                        &model, 
                        *with_projection, 
                        (picture.width, picture.height)
                        );
            },
            Primitive {
                kind: PrimitiveKind::Rectangle(rectangle),
                with_projection,
            } => {
                let (model, points, indicies) = rectangle.get_gfx();
                let geom_data =
                    GeometryData::new(&gl, &points, &indicies).unwrap();
                let fill_color = rectangle.color;
                canvas.render_primitive(
                    &gl,
                    &viewport,
                    frame,
                    &geom_data,
                    &model,
                    (fill_color.x, fill_color.y, fill_color.z),
                    *with_projection,
                    RenderMode::Draw
                );
            }
            Primitive {
                kind: PrimitiveKind::Text(text),
                with_projection: _,
            } => {
                use glyph_brush::{Layout, HorizontalAlign, VerticalAlign};
                text_data.glyph_brush.queue(Section {
                    text: &text.text,
                    scale,
                    screen_position: (text.position.x, text.position.y),
                    // bounds: (w /3.15, h),
                    color: [1.0, 1.0, 1.0, 1.0],
                    layout: Layout::default()
                        .h_align(HorizontalAlign::Center)
                        .v_align(VerticalAlign::Center),
                    ..Section::default()
                });
            }
        }
    }
    canvas.render_text(
        text_data,
        &viewport,
        frame
    );
}


#[derive(Default)]
pub struct UpgradeGUI;

impl<'a> System<'a> for UpgradeGUI {
    type SystemData = (
        (
            Entities<'a>,
            ReadStorage<'a, CharacterMarker>,
            WriteStorage<'a, ShipStats>,
            WriteStorage<'a, MultyLazer>,
            WriteStorage<'a, ShotGun>,
            ReadExpect<'a, red::Viewport>,
        ),
        Write<'a, UI>,
        Write<'a, AppState>,
        Read<'a, Mouse>,
        WriteExpect<'a, PreloadedImages>,
        Read<'a, AvaliableUpgrades>,
        Write<'a, SpawnedUpgrades>,
        WriteExpect<'a, UIState>,
        ReadExpect<'a, Pallete>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                character_markers,
                mut ships_stats,
                mut multiple_lazers,
                mut shotguns,
                viewport,
            ),
            // preloaded_particles,
            mut ui,
            mut app_state,
            mouse,
            preloaded_images,
            avaliable_upgrades,
            mut spawned_upgrades,
            mut ui_state,
            _pallete
        ) = data;
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let (character, ship_stats, _) = (&entities, &mut ships_stats, &character_markers).join().next().unwrap();
        // upgrade UI
        let mut current_upgrade = None;
        let upgrade_button_w = (w/4f32).min(h/2f32);
        let upgrade_button_h = upgrade_button_w;
        let (choose_button_w, choose_button_h) = (w/6f32, h/10f32);
        let shift = upgrade_button_h / 10f32;

        match *app_state {
            AppState::Play(PlayState::Upgrade) => {
                let mut buttons = vec![];
                let upgrades = spawned_upgrades.last();
                if let Some(upgrades) = upgrades {
                    let widget_ids = [Widgets::Upgrade1, Widgets::Upgrade2];
                    for (i, upg_id) in upgrades.iter().enumerate() {
                        let upg = &avaliable_upgrades[*upg_id];
                        let current_point = 
                            Point2::new(
                                w / 2.0 - upgrade_button_w - shift 
                                + i as f32 * (upgrade_button_w + shift), 
                                shift
                            );
                        let upgrade_button = Button::new(
                            current_point,
                            upgrade_button_w, upgrade_button_h, 
                            None,
                            false,
                            Some(upg.image),
                            "".to_string(),
                            widget_ids[i] as usize
                        );
                        ui.primitives.push(
                            Primitive {
                                kind: PrimitiveKind::Text(Text {
                                    position: Point2::new(current_point.x + upgrade_button_h / 2.0, upgrade_button_h + 2.0 * shift),
                                    text: upg.name.clone()
                                }),
                                with_projection: false,
                            }
                        );
                        buttons.push(upgrade_button);
                    }
                }
               let upgrade_selector = Selector {
                    buttons: buttons,
                    id: Widgets::WeaponSelector as usize,
                    mask: None,
                };
                if let Some(selected_id) = upgrade_selector.place_and_check(
                    &mut ui,
                    &*mouse
                ) {
                    match Widgets::try_from(selected_id).expect("unknown widget id") {
                        Widgets::Upgrade1 => {
                            ui_state.choosed_upgrade = Some(0);
                        }
                        Widgets::Upgrade2 => {
                            ui_state.choosed_upgrade = Some(1);
                        }
                        _ => ()
                    }
                }
                let select_upgrade = Button::new(
                    Point2::new(w / 2.0 - choose_button_w - shift, h - 1.0 * choose_button_h),
                    choose_button_w, choose_button_h, 
                    None,
                    false,
                    Some(Image(preloaded_images.upg_bar)),
                    "Upgrade!".to_string(),
                    Widgets::Upgrade as usize
                );

                if spawned_upgrades.len() > 0 {
                    if let Some(upgrade) = ui_state.choosed_upgrade {
                        ui.primitives.push(
                            Primitive {
                                kind: PrimitiveKind::Text(Text {
                                    position: Point2::new(w / 2.0, upgrade_button_h + 4.0 * shift),
                                    text: avaliable_upgrades[upgrade].description.clone()
                                }),
                                with_projection: false,
                            }
                        );
                        if select_upgrade.place_and_check(&mut ui, &*mouse) {
                            current_upgrade = Some(avaliable_upgrades[upgrade].upgrade_type);
                            ui_state.choosed_upgrade = None;
                            spawned_upgrades.pop();
                        }
                    }
                }
                let done_button = Button::new(
                    Point2::new(w / 2.0 + shift, h - 1.0 * choose_button_h),
                    choose_button_w, choose_button_h, 
                    None,
                    false,
                    Some(Image(preloaded_images.upg_bar)),
                    "Done".to_string(),
                    Widgets::Done as usize
                );
                if done_button.place_and_check(&mut ui, &*mouse) {
                    *app_state = AppState::Play(PlayState::Action);
                }
            }
            _ => ()
        }


        match current_upgrade {
            Some(choosed_upgrade) => {
                match choosed_upgrade {
                    UpgradeType::AttackSpeed => {
                        match shotguns.get_mut(character) {
                            Some(gun) => {
                                let recharge_time_millis = (gun.recharge_time.as_millis() as f32 * 0.9) as u64;
                                gun.recharge_time = Duration::from_millis(recharge_time_millis);
                            }
                            None => ()
                        }
                    }
                    UpgradeType::BulletSpeed => {
                        match shotguns.get_mut(character) {
                            Some(gun) => {
                                gun.bullet_speed += 0.1 * BULLET_SPEED_INIT;
                            }
                            None => ()
                        }
                    }
                    UpgradeType::BulletReflection => {
                        if let Some(gun) = shotguns.get_mut(character) {
                            if let Some(ref mut reflection) = gun.reflection {
                                reflection.speed += 0.5;
                            } else {
                                gun.reflection = Some(Reflection{speed: 0.4})
                            }
                        }
                    }
                    UpgradeType::LazerLength => {
                        if let Some(multy_lazer) = multiple_lazers.get_mut(character) {
                            multy_lazer.upgrade_length(0.3);
                        }
                    }                    
                    UpgradeType::ShipSpeed => {
                        ship_stats.thrust_force += 0.1 * THRUST_FORCE_INIT;
                    }
                    UpgradeType::ShipRotationSpeed => {
                        ship_stats.torque += 0.1 * SHIP_ROTATION_SPEED_INIT;
                    }

                    UpgradeType::ShieldRegen => {
                        ship_stats.shield_regen += 1;
                    }
                    UpgradeType::HealthSize => {
                        ship_stats.max_health += 20;
                    }
                    UpgradeType::ShieldSize => {
                        ship_stats.max_shield += 20;
                    }
                }
            }
            None => ()
        }
    }
}

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
            ReadExpect<'a, red::Viewport>,
        ),
        ReadExpect<'a, DevInfo>,
        Write<'a, UI>,
        Read<'a, Progress>,
        Read<'a, Mouse>,
        WriteExpect<'a, PreloadedImages>,
        Write<'a, SpawnedUpgrades>,
        Read<'a, CurrentWave>,
        ReadExpect<'a, Pallete>,
        ReadExpect<'a, MacroGame>
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
                viewport,
            ),
            // preloaded_particles,
            dev_info,
            mut ui,
            progress,
            mouse,
            preloaded_images,
            spawned_upgrades,
            current_wave,
            pallete,
            macro_game,
        ) = data;
        let dims = viewport.dimensions();
        let (w, h) = (dims.0 as f32, dims.1 as f32);
        let d = (w * w + h * h).sqrt();
        //contorls
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
        let stick_size = w / 80.0;
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
        let ctrl_size = stick_size * 10.0;
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
        let move_controller = VecController::new(
            Point2::new(ctrl_size, h - ctrl_size),
            ctrl_size,
            stick_size,
            Image(preloaded_images.circle)
        );
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
        let attack_controller = VecController::new(
            Point2::new(w - ctrl_size, h - ctrl_size),
            ctrl_size,
            stick_size,
            Image(preloaded_images.circle)
        );        
        let (_character, ship_stats, _) = if let Some(value) = (&entities, &mut ships_stats, &character_markers).join().next() {
            value
        } else {
            return
        };
        // move controller
        #[cfg(any(target_os = "ios", target_os = "android", target_os = "emscripten"))]
        {  
            match move_controller.set(
                0,
                &mut ui,
                &touches
            ) {
                Some(dir) => {
                    let (character, _) = (&entities, &character_markers).join().next().unwrap();
                    let (_character_isometry, mut character_velocity) = {
                        let character_body = world
                            .rigid_body(physics.get(character).unwrap().body_handle)
                            .unwrap();
                        (*character_body.position(), *character_body.velocity())
                    };

                    for (iso, _vel, spin, _char_marker) in (
                        &isometries,
                        &mut velocities,
                        &mut spins,
                        &character_markers,
                    ).join()
                    {
                        let player_torque = DT
                            * calculate_player_ship_spin_for_aim(
                                dir,
                                iso.rotation(),
                                spin.0,
                            );
                        spin.0 += player_torque.max(-MAX_TORQUE).min(MAX_TORQUE);
                    }


                    // let rotation = isometries.get(character).unwrap().0.rotation;
                    // let thrust = player_stats.thrust_force * (rotation * Vector3::new(0.0, -1.0, 0.0));
                    let thrust = ship_stats.thrust_force * Vector3::new(dir.x, dir.y, 0.0);
                    *character_velocity.as_vector_mut() += thrust;
                    let character_body = world
                        .rigid_body_mut(physics.get(character).unwrap().body_handle)
                        .unwrap();
                    character_body.set_velocity(character_velocity);
                }
                None => ()
            }

            match attack_controller.set(
                1,
                &mut ui,
                &touches
            ) {
                Some(dir) => {
                    let dir = dir.normalize();
                    let blaster = blasters.get_mut(character);
                    if let Some(blaster) = blaster {
                        if blaster.shoot() {
                            let isometry = *isometries.get(character).unwrap();
                            let position = isometry.0.translation.vector;
                            // let direction = isometry.0 * Vector3::new(0f32, -1f32, 0f32);
                            let velocity_rel = blaster.bullet_speed * dir;
                            let char_velocity = velocities.get(character).unwrap();
                            let projectile_velocity = Velocity::new(
                                char_velocity.0.x + velocity_rel.x,
                                char_velocity.0.y + velocity_rel.y,
                            ) ;
                            sounds_channel.single_write(Sound(preloaded_sounds.shot));
                            let rotation = Rotation2::rotation_between(&Vector2::new(0.0, 1.0), &dir);
                            insert_channel.single_write(InsertEvent::Bullet {
                                kind: EntityType::Player,
                                iso: Point3::new(position.x, position.y, rotation.angle()),
                                velocity: Point2::new(projectile_velocity.0.x, projectile_velocity.0.y),
                                damage: blaster.bullets_damage,
                                owner: character,
                            });
                        }
                    }

                }
                None => ()
            }
        }
        // FPS
        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w/7.0, h / 20.0), 
                    text: format!(
                        "FPS: {}", 
                        dev_info.fps
                    ).to_string()
                }),
                with_projection: false,
            }
        );

        // stats
        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w - w/7.0, h / 20.0), 
                    text: format!(
                        "Score: {}", 
                        progress.score
                    ).to_string()
                }),
                with_projection: false,
            }
        );

        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w - w/7.0, h / 20.0), 
                    text: format!(
                        "$ {}", 
                        macro_game.coins
                    ).to_string()
                }),
                with_projection: false,
            }
        );

        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Text(Text {
                    position: Point2::new(w - w/7.0, h / 7.0 + h / 20.0), 
                    text: format!(
                        "Wave: {}", 
                        current_wave.id
                    ).to_string()
                }),
                with_projection: false,
            }
        );

        let side_bullets_cnt = side_bullet_abilities.count();
        let double_coins_cnt = double_coins_abilities.count();
        let double_exp_cnt = double_exp_abilities.count();
        let icon_size = w/20.0;
        struct Ability {
            pub icon: Image,
            pub text: String
        };
        let mut abilities = vec![];

        if double_coins_cnt > 0 {
            let ability = Ability {
                icon: Image(preloaded_images.double_coin),
                text: format!("x{}", double_coins_cnt).to_string(),
            };
            abilities.push(ability);
        }
        if double_exp_cnt > 0 {
            let ability = Ability {
                icon: Image(preloaded_images.double_exp),
                text: format!("x{}", double_exp_cnt).to_string(),
            };
            abilities.push(ability);
        }
        if side_bullets_cnt > 0 {
            let ability = Ability {
                icon: Image(preloaded_images.side_bullet_ability),
                text: format!("+{}", side_bullets_cnt).to_string()
            };
            abilities.push(ability);
        }

        for (i, ability) in abilities.iter().enumerate() {
            let x_pos = w - w/7.0;
            let y_pos = (i as f32 + 1.0) * h / 7.0 + h / 20.0;
            ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Picture(Picture {
                        position: Point2::new(x_pos, y_pos),
                        width: icon_size,
                        height: icon_size,
                        image: ability.icon
                    }),
                    with_projection: false,
                }
            );
            ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Text(Text {
                        position: Point2::new(x_pos + 2.0 * icon_size, y_pos + icon_size / 2.0), 
                        text: ability.text.clone()
                    }),
                    with_projection: false,
                }
            );            
        }

        let (_character, _) = (&entities, &character_markers).join().next().unwrap();
        // "UI" things
        // experience and level bars
        let experiencebar_w = w / 5.0;
        let experiencebar_h = h / 100.0;
        let experience_position = Point2::new(w/2.0 - experiencebar_w / 2.0, h - h / 20.0);
        let experience_bar = Rectangle {
            position: experience_position,
            width: (progress.experience as f32 / progress.current_max_experience() as f32) * experiencebar_w,
            height: experiencebar_h,
            color: pallete.experience_color.clone()
        };

        let border = d / 200f32;
        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Picture(Picture {
                    position: experience_position + Vector2::new(-border/2.0, -border/2.0),
                    width: experiencebar_w + border,
                    height: experiencebar_h + border,
                    image: Image(preloaded_images.bar)
                }),
                with_projection: false,
            }
        );
        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Picture(Picture {
                    position: experience_position + Vector2::new(-border/2.0, -border/2.0),
                    width: (progress.experience as f32 / progress.current_max_experience() as f32) * experiencebar_w,
                    height: experiencebar_h,
                    image: Image(preloaded_images.bar)
                }),
                with_projection: false,
            }
        );
        ui.primitives.push(
            Primitive {
                kind: PrimitiveKind::Rectangle(experience_bar),
                with_projection: false,
            }
        );
        if spawned_upgrades.len() > 0 {
        // {
            let (upgrade_bar_w, upgrade_bar_h) = (w / 3f32, h / 10.0);
            ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Picture(Picture {
                        position: Point2::new(w / 2.0 - upgrade_bar_w / 2.0, h - h / 20.0 - upgrade_bar_h),
                        width: upgrade_bar_w,
                        height: upgrade_bar_h,
                        image: Image(preloaded_images.upg_bar)
                    }),
                    with_projection: false,
                }
            );
        }
        let (lifebar_w, lifebar_h) = (w/4f32, h/50.0);
        let health_y = h / 40.0;
        let shields_y = health_y + h / 13.0;
        for (life, shield, _character) in (&lifes, &shields, &character_markers).join() {
            {   // upgrade bar
                let border = d / 200f32;
                let (health_back_w, health_back_h) = (lifebar_w + border, lifebar_h + border);
                ui.primitives.push(
                    Primitive {
                        kind: PrimitiveKind::Picture(Picture {
                            position: Point2::new(w/2.0 - health_back_w / 2.0, health_y - border / 2.0),
                            width: health_back_w,
                            height: health_back_h,
                            image: Image(preloaded_images.bar)
                        }),
                        with_projection: false,
                    }
                );

                let (health_back_w, health_back_h) = (lifebar_w + border, lifebar_h + border);
                ui.primitives.push(
                    Primitive {
                        kind: PrimitiveKind::Picture(Picture {
                            position: Point2::new(w/2.0 - health_back_w / 2.0, shields_y - border / 2.0),
                            width: health_back_w,
                            height: health_back_h,
                            image: Image(preloaded_images.bar)
                        }),
                        with_projection: false,
                    }
                );
            }


            let lifes_bar = Rectangle {
                position: Point2::new(w/2.0 - lifebar_w / 2.0, health_y),
                width: (life.0 as f32 / ship_stats.max_health as f32) * lifebar_w,
                height: lifebar_h,
                color: pallete.life_color.clone()
            };
            let shields_bar = Rectangle {
                position: Point2::new(w/2.0 - lifebar_w / 2.0, shields_y),
                width: (shield.0 as f32 / ship_stats.max_shield as f32) * lifebar_w,
                height: lifebar_h,
                color: pallete.shield_color
            };
            ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Rectangle(shields_bar),
                    with_projection: false,
                }
            );
            ui.primitives.push(
                Primitive {
                    kind: PrimitiveKind::Rectangle(lifes_bar),
                    with_projection: false,
                }
            );
        }
    }
}

pub struct RenderingSystem {
    reader: ReaderId<Primitive>,
}

impl RenderingSystem {
    pub fn new(reader: ReaderId<Primitive>) -> Self{
        RenderingSystem {
            reader: reader
        }
    }
}

impl<'a> System<'a> for RenderingSystem {
    type SystemData = (
        (
            Entities<'a>,
            ReadStorage<'a, Isometry>,
            ReadStorage<'a, Velocity>,
            ReadStorage<'a, PhysicsComponent>,
            ReadStorage<'a, CharacterMarker>,
            ReadStorage<'a, ShipMarker>,
            ReadStorage<'a, AsteroidMarker>,
            ReadStorage<'a, LightMarker>,
            ReadStorage<'a, StarsMarker>,
            ReadStorage<'a, NebulaMarker>,
            ReadStorage<'a, PlanetMarker>,
            ReadStorage<'a, BigStarMarker>,
            ReadStorage<'a, Projectile>,
            ReadStorage<'a, ThreadPin<ImageData>>,
            ReadStorage<'a, Image>,
            WriteStorage<'a, Animation>,
            ReadStorage<'a, Size>,
            ReadStorage<'a, Polygon>,
            ReadStorage<'a, Geometry>,
            ReadStorage<'a, CollectableMarker>,
            WriteStorage<'a, ThreadPin<ParticlesData>>,
            ReadStorage<'a, MultyLazer>,
            ReadStorage<'a, Chain>,
            ReadStorage<'a, Rift>
        ),
        WriteExpect<'a, TeleGraph>,
        Read<'a, Mouse>,
        ReadExpect<'a, ThreadPin<red::GL>>,
        ReadExpect<'a, red::Viewport>,
        WriteExpect<'a, Canvas>,
        ReadExpect<'a, PreloadedParticles>,
        Read<'a, World<f32>>,
        Write<'a, EventChannel<Primitive>>,
        Write<'a, UI>,
        WriteExpect<'a, ThreadPin<TextData<'static>>>,
        WriteExpect<'a, GlobalParams>,
        ReadExpect<'a, DevInfo>
    );

    fn run(&mut self, data: Self::SystemData) {
        let (
            (
                entities,
                isometries,
                velocities,
                physics,
                character_markers,
                ship_markers,
                asteroid_markers,
                light_markers,
                stars,
                nebulas,
                planets,
                _big_star_markers,
                projectiles,
                image_datas,
                image_ids,
                mut animations,
                sizes,
                polygons,
                geometries,
                collectables,
                mut particles_datas,
                multy_lazers,
                _chains,
                rifts,
            ),
            mut telegraph,
            mouse,
            gl,
            viewport,
            mut canvas,
            preloaded_particles,
            _world,
            mut primitives_channel,
            mut ui,
            mut text_data,
            mut global_params,
            dev_info
        ) = data;
        let char_pos = if let Some((iso, vel, _)) = (&isometries, &velocities, &character_markers).join().next() {
            canvas.update_observer(
                Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y),
                vel.0.norm() / VELOCITY_MAX,
                Vector2::new(mouse.x01, mouse.y01).normalize()
            );
            iso.0.translation.vector
        } else {
            return
        };
        flame::start("rendering");
        flame::start("clear");
        let mut frame = red::Frame::new(&gl);
        global_params.update();
        frame.set_clear_color(global_params.red.min(1.0), 0.004, 0.0, 1.0);
        // frame.set_clear_color(0.15, 0.004, 0.0, 1.0);
        frame.set_clear_stencil(0);
        // frame.clear_color_and_stencil();
        flame::start("color");
        frame.clear_color();
        flame::end("color");
        flame::start("stencil");
        frame.clear_stencil();
        flame::end("stencil");
        telegraph.update();
        flame::end("clear");
        flame::start("shadow rendering");
        for (_entity, iso, geom, _) in (&entities, &isometries, &geometries, &asteroid_markers).join() {
            let pos = Point2::new(iso.0.translation.vector.x, iso.0.translation.vector.y);
            // light_poly.clip_one((*geom).clone(), pos);
            let rotation = iso.0.rotation.euler_angles().2;
            let rotation = Rotation2::new(rotation);
            let shadow_triangulation = shadow_geometry(
                Point2::new(char_pos.x, char_pos.y),
                (*geom).clone(),
                pos,
                rotation
            );
            if let Some(shadow_triangulation) = shadow_triangulation {
                let geometry_data =  GeometryData::new(
                    &gl, 
                    &shadow_triangulation.points, 
                    &shadow_triangulation.indicies)
                .unwrap();
                let iso = Isometry3::new(iso.0.translation.vector, Vector3::new(0f32, 0f32, 0f32));
                // draw shadows
                canvas
                    .render_geometry(
                        &gl, &viewport,
                        &mut frame,
                        &geometry_data,
                        &iso,
                        RenderMode::StencilWrite,
                        Point3::new(0f32, 0f32, 0f32)
                    );
            }
        }
        flame::end("shadow rendering");
        flame::start("background rendering");
        for (_entity, iso, image, size, _stars) in
            (&entities, &isometries, &image_ids, &sizes, &stars).join() {
            let image_data = image_datas.get(image.0).unwrap();
            canvas
                .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        false,
                        None
                );
        };

        // for (_entity, iso, image, size, _stars) in
        //     (&entities, &isometries, &image_ids, &sizes, &big_star_markers).join() {
        //     let image_data = image_datas.get(image.0).unwrap();
        //     canvas
        //         .render(
        //                 &gl,
        //                 &viewport,
        //                 &mut frame,
        //                 &image_data,
        //                 &iso.0,
        //                 size.0,
        //                 false,
        //                 None
        //         );
        // };
        for (_entity, iso, image, size, _nebula) in
            (&entities, &isometries, &image_ids, &sizes, &nebulas).join() {
            let image_data = image_datas.get(image.0).unwrap();
            canvas
                .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        false,
                        None
                );
        };
        for (_entity, iso, image, size, _planet) in
            (&entities, &isometries, &image_ids, &sizes, &planets).join() {
            let image_data = image_datas.get(image.0).unwrap();
            canvas
                .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        false,
                        None
                );
        };
        flame::end("background rendering");
        flame::start("particles rendering");
        for (entity, particles_data) in (&entities, &mut particles_datas).join() {
            match **particles_data {
                ParticlesData::Explosion(ref mut particles) => {
                    if particles.update() {
                        canvas
                            .render_instancing(
                                &gl,
                                &viewport,
                                &mut frame,
                                &particles.instancing_data,
                                &Isometry3::new(
                                    Vector3::new(0f32, 0f32, 0f32),
                                    Vector3::new(0f32, 0f32, 0f32),
                                )
                            );
                    } else {
                        entities.delete(entity).unwrap();
                    }
            }
                _ => ()
            };
        }

        for (iso, vel, _char_marker) in (&isometries, &velocities, &character_markers).join() {
            let translation_vec = iso.0.translation.vector;
            let mut isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            let pure_isometry = isometry.clone();
            isometry.translation.vector.z = canvas.get_z_shift();
            match **particles_datas
                .get_mut(preloaded_particles.movement)
                .unwrap()
            {
                ParticlesData::MovementParticles(ref mut particles) => {
                    particles.update(1.0 * Vector2::new(-vel.0.x, -vel.0.y));
                     canvas
                        .render_instancing(
                            &gl,
                            &viewport,
                            &mut frame,
                            &particles.instancing_data,
                            &pure_isometry,
                        );
                }
                _ => panic!(),
            };
        }

        flame::end("particles rendering");

        flame::start("other");
        for (_entity, iso, image, size, _light) in
            (&entities, &isometries, &image_ids, &sizes, &light_markers).join()
        {
            let translation_vec = iso.0.translation.vector;
            let isometry = Isometry3::new(translation_vec, Vector3::new(0f32, 0f32, 0f32));
            canvas
                .render(
                    &gl,
                    &viewport,
                    &mut frame,
                    &image_datas.get(image.0).unwrap(),
                    &isometry,
                    size.0,
                    true,
                    Some(red::Blend)
                );
        }


        let mut render_lazer = |
            iso: &Isometry,
            lazer: &Lazer,
            force_rendering: bool,
            rotation
        | {
            if lazer.active || force_rendering {
                let h = lazer.current_distance;
                let w = 0.05f32;
                let positions = vec![
                    Vector2::new(-w / 2.0, 0f32),
                    Vector2::new(w / 2.0, 0f32),
                    Vector2::new(0.0, -h) // hmmmmm, don't know why minus
                ];
                let positions: Vec<Point2> = positions
                    .into_iter()
                    .map(|v: Vector2| Point2::from(rotation * v))
                    .collect();
                let indices = [0u16, 1, 2];
                let geometry_data = GeometryData::new(
                    &gl, &positions, &indices
                ).unwrap();
                canvas.render_geometry(
                    &gl,
                    &viewport,
                    &mut frame,
                    &geometry_data,
                    &iso.0,
                    RenderMode::Draw,
                    Point3::new(1.0, 0.0, 0.0)
                );
            }
        };
        let zero_rotation = Rotation2::new(0.0);
        for (rift, isometry) in (&rifts, &isometries).join() {
            for (lazer, dir) in rift.lazers.iter() {
                let pos = isometry.0.translation.vector;
                let up = Vector2::new(0.0, -1.0);
                let dir = Vector2::new(dir.0, dir.1);
                let rotation = Rotation2::rotation_between(&up, &Vector2::new(dir.x, dir.y));
                let isometry = Isometry3::new(
                    Vector3::new(pos.x, pos.y, pos.z), Vector3::new(0f32, 0f32, rotation.angle())
                );
                render_lazer(&Isometry(isometry), &lazer, false, zero_rotation);
            }
        }
        for (iso, multy_lazer) in (&isometries, &multy_lazers).join() {
            for (angle, lazer) in multy_lazer.iter() {
                // let rotation = Rotation2::new(i as f32 * std::f32::consts::PI / 2.0);
                let rotation = Rotation2::new(angle);
                render_lazer(iso, lazer, false, rotation);
            }
        };
        for (_entity, iso, image, size, _projectile) in
            (&entities, &isometries, &image_ids, &sizes, &projectiles).join()
        {
            canvas
                .render(
                    &gl,
                    &viewport,
                    &mut frame,
                    &image_datas.get(image.0).unwrap(),
                    &iso.0,
                    size.0,
                    true,
                    Some(red::Blend)
                );
        }
        for (_entity, iso, _physics_component, image, size, _ship) in
                    (&entities, &isometries, &physics, &image_ids, &sizes, &ship_markers).join() {
                // let iso2 = world
                //     .rigid_body(physics_component.body_handle)
                //     .unwrap()
                //     .position();
                // let iso = iso2_iso3(iso2);
                let image_data = &image_datas.get(image.0).unwrap();
                canvas
                    .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        true,
                        None
                    )
        }
        flame::end("other");
        flame::start("asteroids rendering");
        for (_entity, iso, _image, _size, polygon, _asteroid) in (
            &entities,
            &isometries,
            &image_ids,
            &sizes,
            &polygons,
            &asteroid_markers,
        )
            .join()
        {
            let polygon = polygon.clone().into_rounded();
            let triangulation = polygon.triangulate();
            let geom_data =
                GeometryData::new(&gl, &triangulation.points, &triangulation.indicies).unwrap();
            canvas
                .render_geometry(
                    &gl, &viewport, 
                    &mut frame, 
                    &geom_data, 
                    &iso.0,
                    RenderMode::Draw,
                    Point3::new(0.8, 0.8, 0.8)
                )
        }
        flame::end("asteroids rendering");
        flame::start("collectables");
        for (iso, size, image, _collectable) in (&isometries, &sizes, &image_ids, &collectables).join() {
            let image_data = image_datas.get(image.0).unwrap();
            canvas
                .render(
                    &gl,
                    &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        true,
                        None
                )
        }
        let _render_line = |
            a: Point2,
            b: Point2
        | {
            let line_width = 0.05;
            let line_length = (b.coords - a.coords).norm();
            let positions = vec![
                Point2::new(-line_width / 2.0, 0f32),
                Point2::new(line_width / 2.0, 0f32),
                Point2::new(-line_width / 2.0, -line_length),
                Point2::new(line_width / 2.0, -line_length)
            ];
            let up = Vector2::new(0.0, -line_length);
            let rotation = Rotation2::rotation_between(&up, &(&b.coords - a.coords));
            let iso = Isometry3::new(
                Vector3::new(a.x, a.y, 0f32), 
                Vector3::new(0f32, 0f32, rotation.angle())
            );
            let indices = [0u16, 1, 2, 0, 2, 3];
            let geometry_data = GeometryData::new(
                &gl, &positions, &indices
            ).unwrap();
            canvas.render_geometry(
                &gl,
                &viewport,
                &mut frame,
                &geometry_data,
                &iso,
                RenderMode::Draw,
                Point3::new(1f32, 1f32, 1f32)
            );
        };
        flame::end("collectables");
        // debug grid drawing
        // for i in 0..nebula_grid.grid.size {
        //     for j in 0..nebula_grid.grid.size {
        //         let ((min_w, max_w), (min_h, max_h)) = nebula_grid.grid.get_rectangle(i, j);
        //         render_line(Point2::new(min_w, min_h), Point2::new(min_w, max_h));
        //         render_line(Point2::new(min_w, max_h), Point2::new(max_w, max_h));
        //         render_line(Point2::new(max_w, max_h), Point2::new(max_w, min_h));                
        //         render_line(Point2::new(max_w, min_h), Point2::new(min_w, min_h));
        //     }
        // }
        flame::start("animation");
        for (iso, size, animation) in (&isometries, &sizes, &mut animations).join() {
            let animation_frame = animation.next_frame();
            if let Some(animation_frame) = animation_frame {
                let image_data = image_datas.get(animation_frame.image.0).unwrap();
                canvas
                    .render(
                        &gl,
                        &viewport,
                        &mut frame,
                        &image_data,
                        &iso.0,
                        size.0,
                        false,
                        Some(red::Blend)
                    )
            };
        };
        flame::end("animation");
        flame::start("primitives rendering");
        primitives_channel.iter_write(ui.primitives.drain(..));
        render_primitives(
            &mouse,
            &mut self.reader,
            &mut frame,
            &image_datas,
            &gl,
            &mut canvas,
            &viewport,
            &mut primitives_channel,
            &mut text_data,
        );
        flame::end("primitives rendering");
        // for (name, span) in time_spans.iter() {
        //     telegraph.insert(name.to_string(), span.evaluate().as_millis() as f32 / 1000.0 * 60.0); // TODO "xFPS" actually
        // }
        flame::end("rendering");
        let spans = flame::spans();
        telegraph.insert("fps".to_string(), dev_info.fps as f32/ 60.0);
        for span in spans.iter() {
            if [
                "rendering".to_string(), 
                "dispatch".to_string(), 
                "insert".to_string(),
                "asteroids".to_string()
            ].contains(&span.name.to_string()) {
                telegraph.insert(span.name.to_string(), span.delta as f32 / 1E9 * 60.0);
            }
            if span.name == "dispatch" {
                for subspan in span.children.iter() {
                    telegraph.insert(subspan.name.to_string(), subspan.delta as f32 / 1E9 * 60.0);
                }
            }
        }
        for name in telegraph.iter_names() {
            if let Some(plot) = telegraph.iter(name.to_string()) {
                render_plot(
                    plot.0,
                    plot.1,
                    14.0, 
                    10.0,
                    &gl,
                    &viewport,
                    &canvas,
                    &mut frame,
                );
            }
        }
    }
}