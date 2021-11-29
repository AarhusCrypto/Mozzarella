use rand::{CryptoRng, Rng};
use scuttlebutt::{AbstractChannel, Block};
use scuttlebutt::ring::R64;
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

        let mut path: [bool; H] = [true, false, true];
        let mut ot_input: [bool; H] = [!path[0], !path[1], !path[2]];
        println!("NOTICE_ME:\tProver still alive 1");
        let mut m: Vec<Block> = ot_receiver.receive(channel, &ot_input, rng)?;
        println!("NOTICE_ME:\tProver still alive 2");
        for i in &m {
            println!("INFO:\tm: {}", i);

        }

        let mut ggm_receiver = ggmReceiver::Receiver::init();
        let v: Vec<R64> = ggm_receiver.gen_eval(channel, rng, &mut path, &mut m)?;
        println!("NOTICE_ME:\tKEK1");

        for i in v {
            println!("R64_OUT:\t {}", i);
        }

        let d:R64 = channel.receive()?;

        println!("DEBUG:\tProver received: {}", d);

        return Ok(vec![Block::default()]);
    }
}