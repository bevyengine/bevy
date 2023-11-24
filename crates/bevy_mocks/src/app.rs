pub struct App;

impl App {
    pub fn add_systems<C, S>(&mut self, _schedule: C, _systems: S) -> &mut Self {
        self
    }
}
