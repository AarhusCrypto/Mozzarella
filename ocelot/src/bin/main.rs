use std::thread::{spawn, JoinHandle};

use rand::{rngs::OsRng, Rng, RngCore};

use scuttlebutt::{channel::unix_channel_pair, Block};
use ocelot::ot::mozzarella::spvole::{prover, verifier};
use ocelot::ot::{KosDeltaSender, Sender as OtSender, KosDeltaReceiver, Receiver as OtReceiver, FixedKeyInitializer};
use std::num::ParseIntError;
use ocelot::Error;
use scuttlebutt::ring::R64;

const GEN_COTS: usize = 1;

fn main() -> Result<(), Error>{

    let tmp: Block = OsRng.gen();
    let tester: Block = OsRng.gen();
    let delta: R64 = R64(tmp.extract_0_u64()); // fyfy, TODO

    // hardcode the two extend calls that we'll need later (since we can't do base vole yet)
    let mut a1 = R64(OsRng.next_u64());
    let mut b1 = R64(OsRng.next_u64());
    let mut tmp = a1;
    println!("TEST:\t a1 = {}", a1);
    tmp *= delta;
    println!("TEST:\t a1*delta {}", a1);
    let mut c1 = tmp;
    c1 += b1;
    println!("TEST:\t c1 {}", c1);


    let mut a2 = R64(OsRng.next_u64());
    let mut b2 = R64(OsRng.next_u64());
    tmp = a2;
    tmp *= delta;
    let mut c2 = tmp;
    c2 += b2;

    let mut prover_base_voles = vec!((a1, c1), (a2, c2));
    let mut verifier_base_voles = vec!(b1, b2);
    let (mut c1, mut c2) = unix_channel_pair();


    let handle: JoinHandle<Result<(), Error>> = spawn(move || {



        let mut kos18_sender = KosDeltaSender::init_fixed_key(&mut c1, tester.into(), &mut OsRng)?;


        let mut verifier_ = verifier::Verifier::init(delta);
        for _ in 0..GEN_COTS {
            verifier_.extend(&mut c1, &mut OsRng, 1, &mut kos18_sender, &mut verifier_base_voles)?;
        }
        return Ok(());
    });

    let mut prover_ = prover::Prover::init();
    let mut kos18_receiver = KosDeltaReceiver::init(&mut c2, &mut OsRng)?;

    for n in 0..GEN_COTS {
        prover_.extend(&mut c2, &mut OsRng, 1, &mut kos18_receiver, &mut prover_base_voles)?;
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