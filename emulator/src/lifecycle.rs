pub trait Lifecycle {
    fn reset(&mut self) {}

    fn init(&mut self) {}

    fn tick(&mut self) {}

    fn halt(&mut self) {}
}
