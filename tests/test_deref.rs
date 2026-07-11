use core::ops::DerefMut;

struct Dummy<B: DerefMut<Target = [u8]>> {
    pool: B,
}

impl<B: DerefMut<Target = [u8]>> Dummy<B> {
    fn access(&mut self) -> u8 {
        self.pool[0]
    }
}

fn main() {}
