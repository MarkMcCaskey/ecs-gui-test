extern crate sdl2;
extern crate specs;

use std::env;
use std::path::Path;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use specs::prelude::*;

#[derive(Debug)]
struct Pos(u32, u32, u32, u32);

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Ord, Eq)]
pub struct Identity(u16);

#[derive(Debug)]
struct Selected(bool);

impl Component for Identity {
    type Storage = VecStorage<Self>;
}

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

impl Component for Selected {
    type Storage = VecStorage<Self>;
}

#[derive(Clone, Debug, Default)]
struct MousePosition(u32, u32);

struct SysA;

#[derive(Debug, Copy, Clone)]
pub enum DrawCommand {
    Select(Identity),
}

#[derive(Debug)]
pub struct Position(u32, u32);

#[derive(Debug)]
enum UiElement {
    Text(Position, String),
    Square(Position, Position, Vec<Identity>),
}

impl UiElement {
    pub fn add_child(&mut self, child_index: Identity) {
        match *self {
            UiElement::Text(..) => (),
            UiElement::Square(_, _, ref mut children) => children.push(child_index),
        }
    }
}

impl<'a> System<'a> for SysA {
    type SystemData = (
        ReadStorage<'a, Identity>,
        WriteStorage<'a, Pos>,
        WriteStorage<'a, Selected>,
        Read<'a, MousePosition>,
        Write<'a, Vec<DrawCommand>>,
    );

    fn run(
        &mut self,
        (ids, mut pos, mut selected, mouse_position, mut draw_commands): Self::SystemData,
    ) {
        println!(
            "Running ECS with mouse at {:?}, {:?}",
            mouse_position.0, mouse_position.1
        );
        for (id, pos, selected) in (&ids, &mut pos, &mut selected).join() {
            *selected = Selected(
                mouse_position.0 >= pos.0 && mouse_position.1 >= pos.1 && mouse_position.0 <= pos.2
                    && mouse_position.1 <= pos.3,
            );
            if selected.0 {
                pos.0 += 5;
                pos.1 += 5;
                pos.2 += 5;
                pos.3 += 5;

                println!("Pushing draw command to select {:?}", id);
                draw_commands.push(DrawCommand::Select(*id));
            }
        }
    }
}

static SCREEN_WIDTH: u32 = 800;
static SCREEN_HEIGHT: u32 = 600;

pub struct ElementCreator {
    latest_id: u16,
    world: World,
    dispatcher: Dispatcher<'static, 'static>,
    ui: Vec<UiElement>,
}

impl ElementCreator {
    pub fn new() -> Self {
        let mut world = World::new();
        world.register::<Identity>();
        world.register::<Pos>();
        world.register::<Selected>();
        world.add_resource(MousePosition(0, 0));
        world.add_resource::<Vec<DrawCommand>>(vec![]);
        let mut dispatcher = DispatcherBuilder::new().with(SysA, "sys_a", &[]).build();
        dispatcher.setup(&mut world.res);

        Self {
            latest_id: 0,
            world,
            dispatcher,
            ui: vec![UiElement::Square(
                Position(0, 0),
                Position(400, 400),
                vec![],
            )],
        }
    }

    pub fn bootstrap_new_entity(&mut self) -> EntityBuilder {
        self.latest_id += 1;
        self.world
            .create_entity()
            .with(Identity(self.latest_id - 1))
    }

    pub fn expose_world(&mut self) -> &mut World {
        &mut self.world
    }

    pub fn add_square(&mut self, (p1, p2): (Position, Position), child_of: Identity) -> Identity {
        let id = self.latest_id;
        self.bootstrap_new_entity()
            .with(Selected(false))
            .with(Pos(p1.0, p1.1, p2.0, p2.1))
            .build();

        self.ui[child_of.0 as usize].add_child(Identity(id));

        self.ui
            .insert(id as usize, UiElement::Square(p1, p2, vec![]));
        Identity(id)
    }

    pub fn draw_to_canvas<T: sdl2::render::RenderTarget>(
        &self,
        canvas: &mut Canvas<T>,
        dc: &mut Vec<DrawCommand>,
    ) {
        canvas.set_draw_color(Color::RGB(0, 0, 0));
        canvas.clear();
        let mut element_stack = vec![0];
        while !element_stack.is_empty() {
            canvas.set_draw_color(Color::RGB(255, 0, 0));
            let current_element = element_stack.pop().unwrap();
            let ui_element = &self.ui[current_element as usize];

            println!("Working on element: {:?}", ui_element);
            for d in dc.iter() {
                match d {
                    DrawCommand::Select(id) => if id.0 == current_element {
                        canvas.set_draw_color(Color::RGB(0, 0, 255));
                    },
                }
            }

            match ui_element {
                UiElement::Square(p1, p2, children) => {
                    for Identity(c) in children {
                        element_stack.push(*c);
                    }
                    canvas.draw_rect(sdl2::rect::Rect::new(
                        p1.0 as i32,
                        p1.1 as i32,
                        p2.0 - p1.0,
                        p2.1 - p1.1,
                    ));
                }
                _ => (),
            }
        }

        canvas.present();
    }

    pub fn dispatch(&mut self) {
        self.dispatcher.dispatch(&mut self.world.res);
    }
}

impl MousePosition {
    pub fn update_position(&mut self, x: u32, y: u32) {
        self.0 = x;
        self.1 = y;
    }
}

fn main() {
    let mut element_creator = ElementCreator::new();

    let this_id = element_creator.add_square((Position(0, 0), Position(50, 50)), Identity(0));
    element_creator.add_square((Position(5, 5), Position(25, 25)), this_id);
    element_creator.add_square((Position(200, 200), Position(300, 300)), Identity(0));

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let window = video_subsystem
        .window("ECS GUI test", SCREEN_WIDTH, SCREEN_HEIGHT)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().software().build().unwrap();
    let texture_creator = canvas.texture_creator();

    'mainloop: loop {
        println!("main loop");
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
                | Event::Quit { .. } => break 'mainloop,
                Event::MouseMotion { x, y, .. } => {
                    element_creator
                        .expose_world()
                        .write_resource::<MousePosition>()
                        .update_position(x as u32, y as u32);
                }

                _ => {}
            }
        }
        element_creator.dispatch();
        let mut dc = element_creator
            .expose_world()
            .read_resource::<Vec<DrawCommand>>()
            .clone();
        element_creator.draw_to_canvas(&mut canvas, &mut dc);
        element_creator
            .expose_world()
            .write_resource::<Vec<DrawCommand>>()
            .clear();
        ::std::thread::sleep_ms(16);
    }
}
