use core::ops::DerefMut;

#[allow(dead_code)]
struct Dummy<B: DerefMut<Target = [u8]>> {
    pool: B,
}

impl<B: DerefMut<Target = [u8]>> Dummy<B> {
    #[allow(dead_code)]
    fn access(&mut self) -> u8 {
        self.pool[0]
    }
}

fn main() {}
