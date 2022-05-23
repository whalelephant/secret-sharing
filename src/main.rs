use ff::Field;
use ff::PrimeField;
use rand_core::RngCore;
use sha2::{Digest, Sha256};
use std::convert::TryInto;

/// This prime field has the greatest 128-bit prime as modulus. Because of the ff crate, each field
/// element is 192bit (3*8 bytes) instead of 128 (2*8) bytes: take care when sampling random bytes.
#[derive(PrimeField)]
#[PrimeFieldModulus = "340282366920938463463374607431768211297"]
#[PrimeFieldGenerator = "7"]
#[PrimeFieldReprEndianness = "little"]
struct FieldElement([u64; 3]);
impl FieldElement {
    /// Create a field element from a u64
    fn new(v: u64) -> Self {
        let mut bytes = [0u8; 3 * 8];
        bytes[0..8].copy_from_slice(&v.to_le_bytes());
        let repr = FieldElementRepr(bytes);
        let elm: FieldElement = PrimeField::from_repr(repr).expect("can create field elm from u64");
        elm
    }

    fn hash(x: &str) -> Self {
        let mut bytes = [0u8; 3 * 8];

        let mut to_hash = x.as_bytes().to_vec();
        let max_fill = 2 * 8;
        loop {
            let mut hasher = Sha256::new();
            hasher.update(&to_hash[..]);
            let hash: [u8; 32] = hasher
                .finalize()
                .as_slice()
                .try_into()
                .expect("Should be a 256-bit hash");
            bytes[..max_fill].clone_from_slice(&hash[..max_fill]);

            // Rejection Sampling
            let repr = FieldElementRepr(bytes);
            if let Some(e) = PrimeField::from_repr(repr) {
                return e;
            }
            to_hash = hash.to_vec();
        }
    }

    fn random() -> Self {
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 3 * 8];
        let max_fill = 2 * 8;
        loop {
            rng.fill_bytes(&mut bytes[0..max_fill]);
            let repr = FieldElementRepr(bytes);
            if let Some(e) = PrimeField::from_repr(repr) {
                return e;
            }
        }
    }
}

/// Represents a Questionnair
#[derive(Debug)]
struct Questionnair {
    questions: Vec<&'static str>,
    tags: Vec<[u8; 32]>,
    points: Vec<FieldElement>,
}

impl Questionnair {
    /// Create random polynomial
    /// Get Share
    fn new(s: FieldElement, questions: Vec<&'static str>, answers: Vec<&'static str>) -> Self {
        let degree = questions.len();
        let polynomial = Polynomial::new(degree as u64, s);
        let shares = polynomial.share(degree as u64);
        let mut tags = Vec::new();
        let mut points = Vec::new();

        for ans in 0..degree {
            let key = FieldElement::hash(&answers[ans]);
            points.push(shares[ans].y + key);

            let tag = tag_from_answer(answers[ans]);
            tags.push(tag);
        }
        Questionnair {
            questions,
            tags,
            points,
        }
    }
}

/// Generates Authenticity tag by H(H(a_i));
fn tag_from_answer(ans: &'static str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(ans);
    let answer_hash = hasher.finalize_reset();
    hasher.update(&answer_hash);
    hasher
        .finalize()
        .as_slice()
        .try_into()
        .expect("Should be a 256-bit hash")
}

/// Lets user answer the questionnair
/// First check if answers are correct
/// Compute shares by calculating keys and decrypt points
/// interpolation of shares to get secret
fn answer(questionnair: Questionnair, answers: Vec<&'static str>) -> Result<FieldElement, String> {
    let mut shares: Vec<Share> = Vec::with_capacity(answers.len());
    // check each answer is
    for (i, ans) in answers.iter().enumerate() {
        let tag = tag_from_answer(*ans);
        if tag != questionnair.tags[i] {
            return Err("Wrong answer".to_string());
        } else {
            // key to decrypt points
            let key = FieldElement::hash(ans);
            shares.push(Share {
                // x point starts at 1, not 0 as f(0) is the secret
                x: FieldElement::new(i as u64 + 1),
                y: questionnair.points[i] - key,
            });
        }
    }
    let interpolated = Polynomial::reconstruct(&shares);
    Ok(interpolated)
}

/// Represents a polynomial over the finite field
#[derive(Debug)]
struct Polynomial {
    degree: u64,
    coefficients: Vec<FieldElement>,
}

/// Represents a point on the polynomial
#[derive(Debug)]
struct Share {
    x: FieldElement,
    y: FieldElement,
}

impl Polynomial {
    /// Create random degree t-1 polynomial with f(0)=s
    fn new(t: u64, s: FieldElement) -> Self {
        let mut coef = vec![s];
        for _ in 1..t - 1 {
            let fe = FieldElement::random();
            coef.push(fe);
        }
        coef.reverse();

        Polynomial {
            degree: t - 1,
            coefficients: coef,
        }
    }

    /// Evaluate polynomial at f(x)
    fn evaluate(&self, x: &FieldElement) -> FieldElement {
        let mut result = self.coefficients[0];
        for i in 1..self.degree as usize {
            result = result * x + self.coefficients[i];
        }
        result
    }

    /// Evaluate polynomial at f(1), .., f(n)
    fn share(&self, n: u64) -> Vec<Share> {
        let mut shares = Vec::new();
        for i in 1..=n {
            let x = FieldElement::new(i);
            let y = self.evaluate(&x);
            shares.push(Share { x, y })
        }
        shares
    }

    /// Compute f(0) by interpolation
    fn reconstruct(shares: &[Share]) -> FieldElement {
        // how do I use a closure?
        // let lagrange_basis_eval = |j: usize, x: FieldElement| unimplemented!();
        let num_keys = shares.len();
        let mut val = FieldElement::zero();
        for i in 0..num_keys - 1 {
            let y = shares[i].y;
            let mut d = FieldElement::one();
            let mut n = FieldElement::one();
            for j in 0..num_keys - 1 {
                if i != j {
                    d *= -shares[j].x;
                    n *= shares[i].x - shares[j].x;
                }
            }
            val += y * d * n.invert().unwrap();
        }
        val
    }
}

pub fn main() {
    let answers = vec!["d", "e", "d", "e", "a"];
    let secret = FieldElement::new(42);

    println!("Creating new Questionnair");
    let questionair = Questionnair::new(secret, vec!["a", "b", "c", "b", "c"], answers.clone());

    println!("Answering Questions");
    let interpolated = answer(questionair, answers).unwrap();

    println!("Checking secret is correct:");
    println!(
        "  {}",
        if interpolated == secret {
            "YAY HURRAY"
        } else {
            "NOPE TRY AGAIN"
        }
    );
}
