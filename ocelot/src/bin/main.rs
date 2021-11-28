use std::thread::{spawn, JoinHandle};

use rand::{rngs::OsRng, Rng};

use scuttlebutt::{channel::unix_channel_pair, Block};
use ocelot::ot::ferret::{FerretReceiver, FerretSender};
use ocelot::ot::mozzarella::spvole::{prover, verifier};
use ocelot::ot::{KosDeltaSender, Sender as OtSender, KosDeltaReceiver, Receiver as OtReceiver, FixedKeyInitializer};
use std::num::ParseIntError;
use ocelot::Error;

const GEN_COTS: usize = 1;

fn main() -> Result<(), Error>{
    let (mut c1, mut c2) = unix_channel_pair();

    let handle: JoinHandle<Result<(), Error>> = spawn(move || {
        let delta: Block = OsRng.gen();
        let mut kos18_sender = KosDeltaSender::init_fixed_key(&mut c1, delta.into(), &mut OsRng)?;


        let mut verifier_ = verifier::Verifier::init(delta);
        for _ in 0..GEN_COTS {
            verifier_.extend(&mut c1, &mut OsRng, 1, &mut kos18_sender)?;
        }
        return Ok(());
    });

    let mut prover_ = prover::Prover::init();
    let mut kos18_receiver = KosDeltaReceiver::init(&mut c2, &mut OsRng)?;

    for n in 0..GEN_COTS {
        prover_.extend(&mut c2, &mut OsRng, 1, &mut kos18_receiver)?;
    }
    handle.join().unwrap();
    return Ok(());






/*    let handle = spawn(move || {
        let delta: Block = OsRng.gen();
        let mut sender = FerretSender::init(delta, &mut c1, &mut OsRng).unwrap();
        for _ in 0..GEN_COTS {
            let _output: Block = sender.cot(&mut c1, &mut OsRng).unwrap();
        }
    });

    let mut receiver = FerretReceiver::init(&mut c2, &mut OsRng).unwrap();
    for n in 0..GEN_COTS {
        let _cot: (bool, Block) = receiver.cot(&mut c2, &mut OsRng).unwrap();
        println!("bool: {}", _cot.0);
        println!("block: {}", _cot.1);
    }
    handle.join().unwrap();
 */
}