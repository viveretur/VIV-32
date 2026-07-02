pub trait Reset {
    fn reset(&mut self);
}

pub trait Init {
    fn init(&mut self);
}

pub trait Tick {
    fn tick(&mut self);
}
