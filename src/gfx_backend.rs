// fork of https://github.com/nukep/glium-sdl2

use std::cell::UnsafeCell;
use std::mem;
use std::ops::Deref;
use std::os::raw::c_void;
use std::rc::Rc;
use crate::types::{*};
use sdl2::video::{Window, WindowBuildError};
use sdl2::VideoSubsystem;

use glium::backend::{Backend, Context, Facade};
use glium::debug;
use glium::IncompatibleOpenGl;
use glium::SwapBuffersError;

// struct Backend {
//     gl_context: GLContext,
// }

#[derive(Debug)]
pub enum GliumSdl2Error {
    WindowBuildError(WindowBuildError),
    ContextCreationError(String),
}

impl From<String> for GliumSdl2Error {
    fn from(s: String) -> GliumSdl2Error {
        return GliumSdl2Error::ContextCreationError(s);
    }
}

impl From<WindowBuildError> for GliumSdl2Error {
    fn from(err: WindowBuildError) -> GliumSdl2Error {
        return GliumSdl2Error::WindowBuildError(err);
    }
}

impl From<IncompatibleOpenGl> for GliumSdl2Error {
    fn from(err: IncompatibleOpenGl) -> GliumSdl2Error {
        GliumSdl2Error::ContextCreationError(err.to_string())
    }
}

impl std::error::Error for GliumSdl2Error {
    fn description(&self) -> &str {
        return match *self {
            GliumSdl2Error::WindowBuildError(ref err) => err.description(),
            GliumSdl2Error::ContextCreationError(ref s) => s,
        };
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            GliumSdl2Error::WindowBuildError(ref err) => err.source(),
            GliumSdl2Error::ContextCreationError(_) => None,
        }
    }
}

impl std::fmt::Display for GliumSdl2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            GliumSdl2Error::WindowBuildError(ref err) => err.fmt(formatter),
            GliumSdl2Error::ContextCreationError(ref err) => err.fmt(formatter),
        }
    }
}

/// Facade implementation for an SDL2 window.
#[derive(Clone)]
pub struct SDL2Facade {
    // contains everything related to the current context and its state
    context: Rc<Context>,

    backend: Rc<SDL2WindowBackend>,
}

impl Facade for SDL2Facade {
    fn get_context(&self) -> &Rc<Context> {
        &self.context
    }
}

impl Deref for SDL2Facade {
    type Target = Context;

    fn deref(&self) -> &Context {
        &self.context
    }
}

impl SDL2Facade {
    pub fn _window(&self) -> &Window {
        self.backend.window()
    }

    pub fn _window_mut(&mut self) -> &mut Window {
        self.backend._window_mut()
    }

    /// Start drawing on the backbuffer.
    ///
    /// This function returns a `Frame`, which can be used to draw on it.
    /// When the `Frame` is destroyed, the buffers are swapped.
    ///
    /// Note that destroying a `Frame` is immediate, even if vsync is enabled.
    pub fn draw(&self) -> glium::Frame {
        glium::Frame::new(
            self.context.clone(),
            self.backend.get_framebuffer_dimensions(),
        )
    }
}

/// An object that can build a facade object.
///
/// This trait is different from `glium::DisplayBuild` because Rust doesn't allow trait
/// implementations on types from external crates, unless the trait is in the same crate as the impl.
/// To clarify, both `glium::DisplayBuild` and `sdl2::video::WindowBuilder` are in different crates
/// than `glium_sdl2`.
pub trait DisplayBuild {
    /// The object that this `DisplayBuild` builds.
    type Facade: glium::backend::Facade;

    /// The type of error that initialization can return.
    type Err;

    /// Build a context and a facade to draw on it.
    ///
    /// Performs a compatibility check to make sure that all core elements of glium
    /// are supported by the implementation.
    fn build_glium(self) -> Result<Self::Facade, Self::Err>
    where
        Self: Sized,
    {
        self.build_glium_debug(Default::default())
    }

    /// Build a context and a facade to draw on it.
    ///
    /// Performs a compatibility check to make sure that all core elements of glium
    /// are supported by the implementation.
    fn build_glium_debug(
        self,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<Self::Facade, Self::Err>;

    /// Build a context and a facade to draw on it
    ///
    /// This function does the same as `build_glium`, except that the resulting context
    /// will assume that the current OpenGL context will never change.
    unsafe fn build_glium_unchecked(self) -> Result<Self::Facade, Self::Err>
    where
        Self: Sized,
    {
        self.build_glium_unchecked_debug(Default::default())
    }

    /// Build a context and a facade to draw on it
    ///
    /// This function does the same as `build_glium`, except that the resulting context
    /// will assume that the current OpenGL context will never change.
    unsafe fn build_glium_unchecked_debug(
        self,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<Self::Facade, Self::Err>;

    // TODO
    // Changes the settings of an existing facade.
    // fn rebuild_glium(self, &Self::Facade) -> Result<(), Self::Err>;
}

impl<'a> DisplayBuild for &'a mut sdl2::video::WindowBuilder {
    type Facade = SDL2Facade;
    type Err = GliumSdl2Error;

    fn build_glium_debug(
        self,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<SDL2Facade, GliumSdl2Error> {
        let backend = Rc::new(SDL2WindowBackend::new(self)?);
        let context = unsafe { Context::new(backend.clone(), true, debug) }?;

        let display = SDL2Facade {
            context: context,
            backend: backend,
        };

        Ok(display)
    }

    unsafe fn build_glium_unchecked_debug(
        self,
        debug: debug::DebugCallbackBehavior,
    ) -> Result<SDL2Facade, GliumSdl2Error> {
        let backend = Rc::new(SDL2WindowBackend::new(self)?);
        let context = Context::new(backend.clone(), false, debug)?;

        let display = SDL2Facade {
            context: context,
            backend: backend,
        };

        Ok(display)
    }
}

pub struct SDL2WindowBackend {
    window: UnsafeCell<sdl2::video::Window>,
    context: sdl2::video::GLContext,
}

impl SDL2WindowBackend {
    fn subsystem(&self) -> &VideoSubsystem {
        let ptr = self.window.get();
        let window: &Window = unsafe { mem::transmute(ptr) };
        window.subsystem()
    }

    fn window(&self) -> &Window {
        let ptr = self.window.get();
        let window: &Window = unsafe { mem::transmute(ptr) };
        window
    }

    fn _window_mut(&self) -> &mut Window {
        let ptr = self.window.get();
        let window: &mut Window = unsafe { mem::transmute(ptr) };
        window
    }

    pub fn new(
        window_builder: &mut sdl2::video::WindowBuilder,
    ) -> Result<SDL2WindowBackend, GliumSdl2Error> {
        let window = window_builder.opengl().build()?;
        let context = window.gl_create_context()?;

        Ok(SDL2WindowBackend {
            window: UnsafeCell::new(window),
            context: context,
        })
    }
}

unsafe impl Backend for SDL2WindowBackend {
    fn swap_buffers(&self) -> Result<(), SwapBuffersError> {
        self.window().gl_swap_window();

        // AFAIK, SDL or `SDL_GL_SwapWindow` doesn't have any way to detect context loss.
        // TODO: Find out if context loss is an issue in SDL2 (especially for the Android port).

        Ok(())
    }

    unsafe fn get_proc_address(&self, symbol: &str) -> *const c_void {
        // Assumes the appropriate context for the window has been set before this call.

        self.subsystem().gl_get_proc_address(symbol) as *const c_void
    }

    fn get_framebuffer_dimensions(&self) -> (u32, u32) {
        let (width, height) = self.window().drawable_size();
        (width as u32, height as u32)
    }

    fn is_current(&self) -> bool {
        self.context.is_current()
    }

    unsafe fn make_current(&self) {
        self.window().gl_make_current(&self.context).unwrap()
    }
}
