// Uncomment these following global attributes to silence most warnings of "low" interest:
/*
#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_variables)]
*/
extern crate nalgebra_glm as glm;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::{mem, os::raw::c_void, ptr};

mod mesh;
mod scene_graph;
mod shader;
mod toolbox;
mod util;
use scene_graph::{Node, SceneNode};

use glutin::event::{
    DeviceEvent,
    ElementState::{Pressed, Released},
    Event, KeyboardInput,
    VirtualKeyCode::{self, *},
    WindowEvent,
};
use glutin::event_loop::ControlFlow;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  byte_size_of_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

// Get a null pointer (equivalent to an offset of 0)
// ptr::null()

unsafe fn create_vao(
    vertices: &Vec<f32>,
    indices: &Vec<u32>,
    colors: &Vec<f32>,
    normals: &Vec<f32>,
) -> u32 {
    unsafe {
        let mut vao = 0;
        let mut vbo = 0;
        let mut ibo = 0;
        let mut cbo = 0;
        let mut nbo = 0;

        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Vertex Buffer Object
        gl::GenBuffers(1, &mut vbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

        gl::BufferData(
            gl::ARRAY_BUFFER,
            byte_size_of_array(vertices),
            pointer_to_array(vertices),
            gl::STATIC_DRAW,
        );

        gl::VertexAttribPointer(
            0,
            3,
            gl::FLOAT,
            gl::FALSE,
            (3 * size_of::<f32>()) as i32,
            offset::<f32>(0),
        );

        gl::EnableVertexAttribArray(0);

        // Index Buffer Object
        gl::GenBuffers(1, &mut ibo);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ibo);

        gl::BufferData(
            gl::ELEMENT_ARRAY_BUFFER,
            byte_size_of_array(indices),
            pointer_to_array(indices),
            gl::STATIC_DRAW,
        );

        // Color Buffer Object
        gl::GenBuffers(1, &mut cbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, cbo);

        gl::BufferData(
            gl::ARRAY_BUFFER,
            byte_size_of_array(colors),
            pointer_to_array(colors),
            gl::STATIC_DRAW,
        );

        gl::VertexAttribPointer(
            1,
            4,
            gl::FLOAT,
            gl::FALSE,
            (4 * size_of::<f32>()) as i32,
            ptr::null(),
        );

        gl::EnableVertexAttribArray(1);

        // Normal Buffer Object
        gl::GenBuffers(1, &mut nbo);
        gl::BindBuffer(gl::ARRAY_BUFFER, nbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            byte_size_of_array(normals),
            pointer_to_array(normals),
            gl::STATIC_DRAW,
        );

        gl::VertexAttribPointer(
            2,
            3,
            gl::FLOAT,
            gl::FALSE,
            (3 * size_of::<f32>()) as i32,
            ptr::null(),
        );

        gl::EnableVertexAttribArray(2);

        gl::BindVertexArray(0);

        vao
    }
}

fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(
            INITIAL_SCREEN_W,
            INITIAL_SCREEN_H,
        ));
    let cb = glutin::ContextBuilder::new().with_vsync(true);
    let windowed_context: glutin::ContextWrapper<glutin::NotCurrent, glutin::window::Window> =
        cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!(
                "{}: {}",
                util::get_gl_string(gl::VENDOR),
                util::get_gl_string(gl::RENDERER)
            );
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!(
                "GLSL\t: {}",
                util::get_gl_string(gl::SHADING_LANGUAGE_VERSION)
            );
        }

        // Load the terrain and create a VAO and node for it
        let terrain_mesh = mesh::Terrain::load("resources/lunarsurface.obj");

        let terrain_vao = unsafe {
            create_vao(
                &terrain_mesh.vertices,
                &terrain_mesh.indices,
                &terrain_mesh.colors,
                &terrain_mesh.normals,
            )
        };

        let terrain_node = SceneNode::from_vao(terrain_vao, terrain_mesh.index_count);

        let helicopter = mesh::Helicopter::load("resources/helicopter.obj");

        let helicopter_body_vao = unsafe {
            create_vao(
                &helicopter.body.vertices,
                &helicopter.body.indices,
                &helicopter.body.colors,
                &helicopter.body.normals,
            )
        };
        let helicopter_door_vao = unsafe {
            create_vao(
                &helicopter.door.vertices,
                &helicopter.door.indices,
                &helicopter.door.colors,
                &helicopter.door.normals,
            )
        };
        let helicopter_main_rotor_vao = unsafe {
            create_vao(
                &helicopter.main_rotor.vertices,
                &helicopter.main_rotor.indices,
                &helicopter.main_rotor.colors,
                &helicopter.main_rotor.normals,
            )
        };
        let helicopter_tail_rotor_vao = unsafe {
            create_vao(
                &helicopter.tail_rotor.vertices,
                &helicopter.tail_rotor.indices,
                &helicopter.tail_rotor.colors,
                &helicopter.tail_rotor.normals,
            )
        };

        let mut helicopters: Vec<Node> = Vec::new();
        let helicopter_count = 5;

        // Create multiple helicopters
        for _i in 0..helicopter_count {
            let mut helicopter_root_node = SceneNode::new();

            let mut helicopter_body_node =
                SceneNode::from_vao(helicopter_body_vao, helicopter.body.index_count);
            helicopter_body_node.reference_point = glm::vec3(0.0, 0.0, 0.0);

            let helicopter_door_node =
                SceneNode::from_vao(helicopter_door_vao, helicopter.door.index_count);
            let mut helicopter_main_rotor_node =
                SceneNode::from_vao(helicopter_main_rotor_vao, helicopter.main_rotor.index_count);
            helicopter_main_rotor_node.reference_point = glm::vec3(0.0, 0.0, 0.0);

            let mut helicopter_tail_rotor_node =
                SceneNode::from_vao(helicopter_tail_rotor_vao, helicopter.tail_rotor.index_count);
            helicopter_tail_rotor_node.reference_point = glm::vec3(0.35, 2.3, 10.4);

            helicopter_body_node.add_child(&helicopter_door_node);
            helicopter_body_node.add_child(&helicopter_main_rotor_node);
            helicopter_body_node.add_child(&helicopter_tail_rotor_node);

            helicopter_root_node
                .add_child(&helicopter_body_node);

            helicopters.push(helicopter_root_node);
        }

        let mut root_node = SceneNode::new();

        root_node.add_child(&terrain_node);
        for helicopter in helicopters.iter() {
            root_node.add_child(helicopter);
        }

        let simple_shader = unsafe {
            shader::ShaderBuilder::new()
                .attach_file("shaders/simple.vert")
                .attach_file("shaders/simple.frag")
                .link()
        };

        unsafe { simple_shader.activate() };

        // Excercise2 Task4 Part b)
        let projection_matrix =
            glm::perspective(window_aspect_ratio, 45.0_f32.to_radians(), 1.0, 1000.0);

        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut previous_frame_time = first_frame_time;

        // Excercise2 Task4 Part c) (a)
        let mut camera_position = glm::vec3(0.0, 0.0, 5.0);
        let mut camera_rotation_x = 0.0_f32;
        let mut camera_rotation_y = 0.0_f32;

        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(previous_frame_time).as_secs_f32();
            previous_frame_time = now;

            // Excercise2 Task4 Part c) (b)
            if let Ok(keys) = pressed_keys.lock() {
                let move_speed = 50.0 * delta_time;
                let rotate_speed = 90.0_f32.to_radians() * delta_time;

                for key in keys.iter() {
                    match key {
                        // Translation keys (WASD + Space + LShift)
                        VirtualKeyCode::W => {
                            camera_position.z -= move_speed;
                        }
                        VirtualKeyCode::S => {
                            camera_position.z += move_speed;
                        }
                        VirtualKeyCode::A => {
                            camera_position.x -= move_speed;
                        }
                        VirtualKeyCode::D => {
                            camera_position.x += move_speed;
                        }
                        VirtualKeyCode::Space => {
                            camera_position.y += move_speed;
                        }
                        VirtualKeyCode::LShift => {
                            camera_position.y -= move_speed;
                        }

                        // Rotation keys (Arrow keys)
                        VirtualKeyCode::Up => {
                            camera_rotation_x += rotate_speed;
                        }
                        VirtualKeyCode::Down => {
                            camera_rotation_x -= rotate_speed;
                        }
                        VirtualKeyCode::Left => {
                            camera_rotation_y -= rotate_speed;
                        }
                        VirtualKeyCode::Right => {
                            camera_rotation_y += rotate_speed;
                        }

                        _ => {}
                    }
                }
            }

            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {
                // == // Optionally access the accumulated mouse movement between
                // == // frames here with delta.0 and delta.1

                *delta = (0.0, 0.0); // reset when done
            }

            // == // Please compute camera transforms here (exercise 2 & 3)

            // Iterate over all helicopters and animate them
            for (i, helicopter) in helicopters.iter_mut().enumerate() {
                let animation_offset = i as f32 * 0.8;
                let helicopter_elapsed = elapsed + animation_offset;

                let body_node = helicopter.get_child(0);

                let main_rotor_node = body_node.get_child(1);
                main_rotor_node.rotation.y = helicopter_elapsed * 10.0;

                let tail_rotor_node = body_node.get_child(2);
                tail_rotor_node.rotation.x = helicopter_elapsed * 20.0;

                let heading = toolbox::simple_heading_animation(helicopter_elapsed);
                body_node.position.x = heading.x;
                body_node.position.z = heading.z;
                body_node.rotation.z = heading.roll;
                body_node.rotation.y = heading.yaw;
                body_node.rotation.x = heading.pitch;
            }

            let rotation_x_matrix = glm::rotation(camera_rotation_x, &glm::vec3(1.0, 0.0, 0.0));
            let rotation_y_matrix = glm::rotation(camera_rotation_y, &glm::vec3(0.0, 1.0, 0.0));
            let rotation_matrix = rotation_y_matrix * rotation_x_matrix;
            let translation_matrix = glm::translate(&glm::Mat4::identity(), &-camera_position);
            let view_matrix = rotation_matrix * translation_matrix;

            let combined_matrix = projection_matrix * view_matrix;

            unsafe fn draw_scene(
                node: &scene_graph::SceneNode,
                view_projection_matrix: &glm::Mat4,
                transformation_so_far: &glm::Mat4,
                shader_program: u32,
            ) {
                let local_translation = glm::translation(&node.position);
                let local_rotation = glm::rotation(node.rotation.x, &glm::vec3(1.0, 0.0, 0.0))
                    * glm::rotation(node.rotation.y, &glm::vec3(0.0, 1.0, 0.0))
                    * glm::rotation(node.rotation.z, &glm::vec3(0.0, 0.0, 1.0));
                let local_scaling = glm::scaling(&node.scale);

                let translation_to_origin = glm::translation(&-node.reference_point);
                let translation_back = glm::translation(&node.reference_point);

                let local_transform = local_translation
                    * translation_back
                    * local_rotation
                    * translation_to_origin
                    * local_scaling;

                let combined_transform = transformation_so_far * local_transform;

                let mvp_matrix = view_projection_matrix * combined_transform;

                let transform_loc = gl::GetUniformLocation(
                    shader_program,
                    b"transformMatrix\0".as_ptr() as *const _,
                );
                gl::UniformMatrix4fv(transform_loc, 1, gl::FALSE, mvp_matrix.as_ptr());

                let model_loc =
                    gl::GetUniformLocation(shader_program, b"modelMatrix\0".as_ptr() as *const _);
                gl::UniformMatrix4fv(model_loc, 1, gl::FALSE, combined_transform.as_ptr());

                if node.vao_id != 0 {
                    gl::BindVertexArray(node.vao_id);
                    gl::DrawElements(
                        gl::TRIANGLES,
                        node.index_count,
                        gl::UNSIGNED_INT,
                        std::ptr::null(),
                    );
                    gl::BindVertexArray(0);
                }

                for &child in &node.children {
                    draw_scene(
                        &*child,
                        view_projection_matrix,
                        &combined_transform,
                        shader_program,
                    );
                }
            }

            unsafe {
                // == // Issue the necessary gl:: commands to draw your scene here

                gl::ClearColor(0.035, 0.046, 0.078, 1.0);
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                draw_scene(
                    &root_node,
                    &combined_matrix,
                    &glm::identity::<f32, 4>(),
                    simple_shader.program_id,
                );

                context.swap_buffers().unwrap();
            }
        }
    });

    // == //
    // == // From here on down there are only internals.
    // == //

    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::Resized(physical_size),
                ..
            } => {
                println!(
                    "New window size received: {}x{}",
                    physical_size.width, physical_size.height
                );
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: key_state,
                                virtual_keycode: Some(keycode),
                                ..
                            },
                        ..
                    },
                ..
            } => {
                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        }
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => {
                        *control_flow = ControlFlow::Exit;
                    }
                    Q => {
                        *control_flow = ControlFlow::Exit;
                    }
                    _ => {}
                }
            }
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => {}
        }
    });
}
