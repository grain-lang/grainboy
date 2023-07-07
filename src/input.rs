
use winit::event::ElementState;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InputState {
    inner: u8,
}
#[allow(non_upper_case_globals)]
impl InputState {
    const Released: Self = Self { inner: 0 };
    const JustPressed: Self = Self { inner: 1 };
    const Pressed: Self = Self { inner: 2 };
    const JustReleased: Self = Self { inner: 3 };
    #[allow(unused)]
    pub fn just_released(&self) -> bool {
        *self == Self::JustReleased
    }
    #[allow(unused)]
    pub fn released(&self) -> bool {
        *self == Self::JustReleased || *self == Self::Released
    }
    #[allow(unused)]
    pub fn just_pressed(&self) -> bool {
        *self == Self::JustPressed
    }
    #[allow(unused)]
    pub fn pressed(&self) -> bool {
        *self == Self::JustPressed || *self == Self::Pressed
    }
    pub fn next(self, state: ElementState) -> Self {
        match (self, state) {
            (InputState::Released, ElementState::Released) => InputState::Released,
            (InputState::Released, ElementState::Pressed) => InputState::JustPressed,
            (InputState::JustReleased, ElementState::Released) => InputState::Released,
            (InputState::JustReleased, ElementState::Pressed) => InputState::JustPressed,
            (InputState::JustPressed, ElementState::Released) => InputState::JustReleased,
            (InputState::JustPressed, ElementState::Pressed) => InputState::Pressed,
            (InputState::Pressed, ElementState::Released) => InputState::JustReleased,
            (InputState::Pressed, ElementState::Pressed) => InputState::Pressed,
            _ => InputState::Released,
        }
    }
    pub fn main_events_cleared(&mut self) {
        *self = match *self {
            InputState::JustPressed => InputState::Pressed,
            InputState::JustReleased => InputState::Released,
            a => a,
        };
    }
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MouseButtons<T: Copy> {
    pub left: T,
    pub right: T,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Buttons<T: Copy> {
    pub up: T,
    pub down: T,
    pub left: T,
    pub right: T,
    pub a: T,
    pub b: T,
    pub x: T,
    pub y: T,
    pub start: T,
    pub select: T,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UserInput {
    pub buttons: Buttons<InputState>,
    pub mouse: MouseButtons<InputState>,
    pub wheel: [i32; 2],
    pub cursor: [i32; 2],
}
impl UserInput {
    pub fn new() -> Self {
        Self {
            buttons: Buttons {
                up: InputState::Released,
                down: InputState::Released,
                left: InputState::Released,
                right: InputState::Released,
                a: InputState::Released,
                b: InputState::Released,
                x: InputState::Released,
                y: InputState::Released,
                start: InputState::Released,
                select: InputState::Released,
            },
            mouse: MouseButtons {
                left: InputState::Released,
                right: InputState::Released,
            },
            wheel: [0; 2],
            cursor: [0; 2],
        }
    }
    pub fn main_events_cleared(&mut self) {
        self.buttons.up.main_events_cleared();
        self.buttons.down.main_events_cleared();
        self.buttons.left.main_events_cleared();
        self.buttons.right.main_events_cleared();
        self.buttons.a.main_events_cleared();
        self.buttons.b.main_events_cleared();
        self.buttons.x.main_events_cleared();
        self.buttons.y.main_events_cleared();
        self.buttons.start.main_events_cleared();
        self.buttons.select.main_events_cleared();

        // Mouse events don't happen every frame
        // So we have to manually transition from the Just* InputStates
        if let InputState::JustPressed = self.mouse.left {
            self.mouse.left = self.mouse.left.next(ElementState::Pressed);
        }
        if let InputState::JustReleased = self.mouse.left {
            self.mouse.left = self.mouse.left.next(ElementState::Released);
        }
        if let InputState::JustPressed = self.mouse.right {
            self.mouse.right = self.mouse.right.next(ElementState::Pressed);
        }
        if let InputState::JustReleased = self.mouse.right {
            self.mouse.right = self.mouse.right.next(ElementState::Released);
        }
        // Reset mouse wheel delta
        self.wheel = [0; 2];
    }
}
