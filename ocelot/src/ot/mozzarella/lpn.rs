use rand::{
    distributions::{Distribution, Standard},
    CryptoRng,
    Rng,
    SeedableRng,
};
use rayon::prelude::*;
use scuttlebutt::{ring::NewRing, AesRng, Block};

// Z64 Local Linear Code with parameter D
// pub struct LLCode<const ROWS: usize, const COLS: usize, const D: usize> {
pub struct LLCode<RingT> {
    pub rows: usize,
    pub columns: usize,
    pub nonzero_entries_per_column: usize,
    indices: Vec<(usize, RingT)>,
}

// columns have length of rows
// impl<const ROWS: usize, const COLS: usize, const D: usize> LLCode<ROWS, COLS, D> {
impl<RingT> LLCode<RingT>
where
    RingT: NewRing,
    Standard: Distribution<RingT>,
{
    pub fn from_seed(
        rows: usize,
        columns: usize,
        nonzero_entries_per_column: usize,
        seed: Block,
    ) -> Self {
        let mut rng = AesRng::from_seed(seed);
        Self::gen(rows, columns, nonzero_entries_per_column, &mut rng)
    }

    #[inline]
    fn gen_column<R: Rng>(rng: &mut R, rows: usize, column: &mut [(usize, RingT)]) {
        assert!(column.len() > 0);
        // assert!(self.rows > 0);
        let mut count = 1;
        column[0].0 = rng.gen_range(0, rows);

        while count < column.len() {
            let new_index: usize = rng.gen_range(0, rows);
            if column[..count].iter().all(|&x| x.0 != new_index) {
                column[count].0 = new_index;
                count += 1;
            }
        }
        for i in 0..column.len() {
            column[i].1 = rng.gen();
        }

        column.sort_by_key(|x| x.0);
    }

    pub fn gen<R: Rng + CryptoRng>(
        rows: usize,
        columns: usize,
        nonzero_entries_per_column: usize,
        rng: &mut R,
    ) -> Self {
        let mut code = LLCode {
            rows,
            columns,
            nonzero_entries_per_column,
            indices: vec![(0, RingT::default()); columns * nonzero_entries_per_column],
        };
        for col_i in 0..columns {
            Self::gen_column(
                rng,
                code.rows,
                &mut code.indices[col_i * code.nonzero_entries_per_column
                    ..(col_i + 1) * code.nonzero_entries_per_column],
            );
        }
        println!("COLS:\t {}", columns);
        code
    }

    // TODO: Can likely be made more efficient somehow?
    pub fn mul(&self, v: &[RingT]) -> Vec<RingT> {
        assert_eq!(v.len(), self.rows);
        (self
            .indices
            .par_chunks_exact(self.nonzero_entries_per_column)
            .map(|col| {
                let mut cord: RingT = RingT::default();
                for i in col {
                    cord += i.1 * v[i.0];
                }
                cord.reduce()
            }))
        .collect()
    }

    // takes the indices of the code (A) and adds them to elements of a.
    pub fn mul_add(&self, v: &[RingT], a: &[RingT]) -> Vec<RingT> {
        assert_eq!(v.len(), self.rows);
        assert_eq!(a.len(), self.columns);
        (self
            .indices
            .par_chunks_exact(self.nonzero_entries_per_column)
            .enumerate()
            .map(|(j, col)| {
                let mut cord: RingT = RingT::default();
                for i in col {
                    cord += i.1 * v[i.0];
                }
                (cord + a[j]).reduce()
            }))
        .collect()
    }
}
