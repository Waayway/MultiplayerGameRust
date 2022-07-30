mod window;
mod texture;
mod camera;

fn main() {
    pollster::block_on(window::run());
}
