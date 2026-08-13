pub struct Config<'b> {
    uuid: &'b [u8],
}

impl<'b> Config<'b> {
    pub fn new(uuid: &'b [u8]) -> Self {
        Self { uuid }
    }

    pub fn uuid(&self) -> &[u8] {
        self.uuid
    }
}
