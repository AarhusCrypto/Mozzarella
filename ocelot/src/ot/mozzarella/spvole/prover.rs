use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use crate::Error;
use crate::ot::mozzarella::ggm::receiver as ggmReceiver;
use crate::ot::{CorrelatedReceiver, RandomReceiver, Receiver as OtReceiver};

pub struct Prover {}

impl Prover {

    pub fn init() -> Self {
        Self{}
    }

    pub fn extend<
        OT: OtReceiver<Msg = Block> + CorrelatedReceiver + RandomReceiver,
        C: AbstractChannel, RNG: CryptoRng + Rng>(
        &mut self,
        channel: &mut C,
        rng: &mut RNG,
        num: usize, // number of repetitions
        ot_receiver: &mut OT,
    ) -> Result<Vec<Block>, Error> {
        println!("INFO:\tProver called!");

        const N: usize = 8;
        const H: usize = 3;

        let mut path: [bool; H] = [true, true, false];
        let mut ot_input: [bool; H] = [false, false, true]; // the input to the OT function
        let mut m: Vec<Block> = ot_receiver.receive(channel, &ot_input, rng)?;
        for i in &m {
            println!("INFO:\tm: {}", i);

        }

        let mut ggm_receiver = ggmReceiver::Receiver::init();
        ggm_receiver.gen_eval(channel, rng, &mut path, &mut m);

        return Ok(vec![Block::default()]);
    }
}